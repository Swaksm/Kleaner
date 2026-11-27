//! Garde de sécurité : toute suppression ou mise à la Corbeille passe par ici.
//!
//! - `check_delete` : suppression définitive, autorisée uniquement *à l'intérieur*
//!   des racines déclarées dans le catalogue (allowlist).
//! - `check_trash` : mise à la Corbeille d'un fichier choisi par l'utilisateur,
//!   refusée dans les emplacements système (blocklist).

use std::fmt;
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::{Component, Path, PathBuf, Prefix};

const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

#[derive(Debug, PartialEq, Eq)]
pub enum GuardError {
    Relative,
    ParentDir,
    NotLocal,
    NotFound,
    Link,
    OutsideRoots,
    Protected,
    SystemFile,
}

impl fmt::Display for GuardError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            Self::Relative => "relative path",
            Self::ParentDir => "path containing '..'",
            Self::NotLocal => "network or non-local path",
            Self::NotFound => "path not found",
            Self::Link => "link or junction on the path",
            Self::OutsideRoots => "outside allowed locations",
            Self::Protected => "protected location",
            Self::SystemFile => "system file",
        };
        f.write_str(msg)
    }
}

/// Chemin canonique (noms longs, casse et `.` finaux résolus par Windows) + clé en minuscules.
struct Canon {
    path: PathBuf,
    key: String,
}

/// Vérifications lexicales, faites avant tout accès disque (pas d'accès réseau sur un chemin UNC).
fn check_lexical(p: &Path) -> Result<(), GuardError> {
    if !p.is_absolute() {
        return Err(GuardError::Relative);
    }
    match p.components().next() {
        Some(Component::Prefix(px)) if matches!(px.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) => {}
        _ => return Err(GuardError::NotLocal),
    }
    if p.components().any(|c| c == Component::ParentDir) {
        return Err(GuardError::ParentDir);
    }
    Ok(())
}

fn canon(p: &Path) -> Result<Canon, GuardError> {
    check_lexical(p)?;
    let path = std::fs::canonicalize(p).map_err(|_| GuardError::NotFound)?;
    let key = path.to_string_lossy().to_lowercase();
    Ok(Canon { path, key })
}

/// `child` strictement sous `parent` (clés canoniques en minuscules).
fn inside(child: &str, parent: &str) -> bool {
    let parent = parent.trim_end_matches('\\');
    child.len() > parent.len() + 1 && child.starts_with(parent) && child.as_bytes()[parent.len()] == b'\\'
}

/// Refuse si le chemin ou l'un de ses parents est un lien symbolique ou une jonction.
pub fn no_links(p: &Path) -> Result<(), GuardError> {
    for a in p.ancestors() {
        // `is_symlink` : lien, jonction ou point de montage (« substitut de nom »). Un dossier OneDrive ou Dropbox
        // porte aussi un point d'analyse, mais ne renvoie nulle part : il n'est pas refusé.
        if let Ok(m) = std::fs::symlink_metadata(a)
            && m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            && m.file_type().is_symlink()
        {
            return Err(GuardError::Link);
        }
    }
    Ok(())
}

/// Autorise la suppression définitive de `p` seulement s'il est strictement sous une des `roots`.
pub fn check_delete(p: &Path, roots: &[PathBuf]) -> Result<PathBuf, GuardError> {
    check_delete_root(p, roots).map(|(path, _)| path)
}

/// Comme `check_delete`, et renvoie aussi la clé canonique de la racine qui contient `p`.
fn check_delete_root(p: &Path, roots: &[PathBuf]) -> Result<(PathBuf, String), GuardError> {
    check_lexical(p)?;
    no_links(p)?;
    let c = canon(p)?;
    for root in roots {
        if let Ok(r) = canon(root)
            && inside(&c.key, &r.key)
        {
            return Ok((c.path, r.key));
        }
    }
    Err(GuardError::OutsideRoots)
}

/// Fichier (ou dossier) ouvert pour être supprimé, sans suivre de lien. Fermé à la fin.
struct Opened(windows_sys::Win32::Foundation::HANDLE);

impl Drop for Opened {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

/// Ouvre `p` puis vérifie, sur le fichier ouvert lui-même, qu'il n'est pas un lien, qu'il est du bon type et que son
/// vrai chemin (donné par Windows pour ce fichier, pas recalculé depuis le texte) est strictement sous une des `roots`.
/// Ce qui est vérifié est donc exactement ce qui sera supprimé : un dossier parent remplacé par une jonction entre
/// l'analyse et la suppression mène ailleurs, et ce chemin-là est refusé.
fn open_inside(p: &Path, roots: &[PathBuf], dir: bool) -> Option<Opened> {
    // Vérifications de texte d'abord (relatif, réseau, « .. », liens sur le chemin, hors des racines).
    let (_, root) = check_delete_root(p, roots).ok()?;
    open_checked(p, &root, dir)
}

/// La partie de `open_inside` faite sur le fichier ouvert : seule, elle doit suffire à refuser un chemin
/// dont un dossier parent mène ailleurs (c'est l'état où se trouve le disque après une substitution).
/// `root` : clé canonique de la racine (voir `canon`).
fn open_checked(p: &Path, root: &str, dir: bool) -> Option<Opened> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateFileW, DELETE, FILE_ATTRIBUTE_DIRECTORY, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_NAME_NORMALIZED, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle,
        GetFinalPathNameByHandleW, OPEN_EXISTING, VOLUME_NAME_DOS,
    };
    let wide: Vec<u16> = p.as_os_str().encode_wide().chain([0]).collect();
    let h = unsafe {
        CreateFileW(
            wide.as_ptr(),
            DELETE | FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            // Mode sauvegarde seulement pour un dossier (Windows l'exige pour l'ouvrir) : un fichier garde ses droits,
            // comme avec la suppression classique.
            FILE_FLAG_OPEN_REPARSE_POINT | if dir { FILE_FLAG_BACKUP_SEMANTICS } else { 0 },
            std::ptr::null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return None;
    }
    let opened = Opened(h);
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(h, &mut info) } == 0 {
        return None;
    }
    let attrs = info.dwFileAttributes;
    if attrs & FILE_ATTRIBUTE_REPARSE_POINT != 0 || (attrs & FILE_ATTRIBUTE_DIRECTORY != 0) != dir {
        return None;
    }
    let mut buf = vec![0u16; 1024];
    let mut n = unsafe { GetFinalPathNameByHandleW(h, buf.as_mut_ptr(), buf.len() as u32, FILE_NAME_NORMALIZED | VOLUME_NAME_DOS) } as usize;
    if n >= buf.len() {
        buf = vec![0u16; n + 1];
        n = unsafe { GetFinalPathNameByHandleW(h, buf.as_mut_ptr(), buf.len() as u32, FILE_NAME_NORMALIZED | VOLUME_NAME_DOS) } as usize;
    }
    if n == 0 || n >= buf.len() {
        return None;
    }
    // Même forme que `canon` (« \\?\c:\… » en minuscules) : même fonction de Windows des deux côtés.
    let key = String::from_utf16_lossy(&buf[..n]).to_lowercase();
    inside(&key, root).then_some(opened)
}

/// Supprime un fichier ouvert. Windows 10 1809+ : suppression immédiate, même en lecture seule.
/// Plus ancien : suppression classique, attribut lecture seule retiré au besoin.
fn dispose(h: &Opened, p: &Path) -> bool {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE, FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
        FILE_DISPOSITION_INFO_EX, FileDispositionInfoEx, SetFileInformationByHandle,
    };
    let ex = FILE_DISPOSITION_INFO_EX {
        Flags: FILE_DISPOSITION_FLAG_DELETE | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS | FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE,
    };
    let done = unsafe {
        SetFileInformationByHandle(h.0, FileDispositionInfoEx, (&ex as *const FILE_DISPOSITION_INFO_EX).cast(), size_of::<FILE_DISPOSITION_INFO_EX>() as u32)
    } != 0;
    done || dispose_classic(h, p)
}

/// Suppression classique (Windows 10 avant 1809) : effective à la fermeture du fichier.
fn dispose_classic(h: &Opened, p: &Path) -> bool {
    use windows_sys::Win32::Storage::FileSystem::{FILE_DISPOSITION_INFO, FileDispositionInfo, SetFileInformationByHandle};
    let old = FILE_DISPOSITION_INFO { DeleteFile: true };
    let classic = || {
        let r = unsafe { SetFileInformationByHandle(h.0, FileDispositionInfo, (&old as *const FILE_DISPOSITION_INFO).cast(), size_of::<FILE_DISPOSITION_INFO>() as u32) };
        r != 0
    };
    if classic() {
        return true;
    }
    // Lecture seule (Windows plus ancien) : le fichier reste ouvert, seul son attribut change.
    // shortcut: attribut retiré par le chemin, pas par le fichier ouvert (il faudrait FILE_WRITE_ATTRIBUTES à l'ouverture) ;
    // au pire un autre fichier perd sa lecture seule, rien n'est supprimé ailleurs. Seulement avant Windows 10 1809.
    match fs::symlink_metadata(p) {
        Ok(m) if m.permissions().readonly() => {
            let mut perms = m.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            fs::set_permissions(p, perms).is_ok() && classic()
        }
        _ => false,
    }
}

/// Supprime définitivement le fichier `p` (ou le dossier vide si `dir`) s'il est strictement sous une des `roots`,
/// vérifié sur le fichier ouvert (voir `open_inside`). Liens et fichiers verrouillés : `false`.
pub fn delete_inside(p: &Path, roots: &[PathBuf], dir: bool) -> bool {
    open_inside(p, roots, dir).is_some_and(|h| dispose(&h, p))
}

/// Même décision que `delete_inside`, sans rien supprimer (comparaison sur la vraie machine).
pub fn would_delete(p: &Path, roots: &[PathBuf], dir: bool) -> bool {
    open_inside(p, roots, dir).is_some()
}

/// Autorise le déplacement de `p` vers un autre disque seulement s'il est strictement sous une des `roots`
/// (dossiers personnels : Bureau, Documents… ; pour un retour, le dossier « Kleaner moved » du disque).
pub fn check_move(p: &Path, roots: &[PathBuf]) -> Result<PathBuf, GuardError> {
    check_delete(p, roots)
}

/// Emplacements interdits à la mise à la Corbeille.
pub struct Protected {
    /// Interdits, eux et tout ce qu'ils contiennent.
    subtrees: Vec<String>,
    /// Interdits en tant que dossier entier, leur contenu reste autorisé.
    exact: Vec<String>,
    /// Exceptions dans un sous-arbre interdit : leur contenu est autorisé (raccourcis du menu Démarrer commun).
    allowed: Vec<String>,
}

impl Protected {
    pub fn from_env() -> Self {
        let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
        let subtrees = ["SystemRoot", "ProgramFiles", "ProgramFiles(x86)", "ProgramData"]
            .iter()
            .filter_map(|k| var(k))
            .collect::<Vec<_>>();
        let mut exact = Vec::new();
        if let Some(home) = var("USERPROFILE") {
            if let Some(users) = home.parent() {
                exact.push(users.to_path_buf());
            }
            for d in ["Desktop", "Documents", "Downloads", "Pictures", "Music", "Videos", "AppData"] {
                exact.push(home.join(d));
            }
            // Les racines d'AppData elles-mêmes (leurs sous-dossiers restent possibles : restes de programmes).
            for d in ["AppData\\Local", "AppData\\Roaming", "AppData\\LocalLow", "AppData\\Local\\Programs"] {
                exact.push(home.join(d));
            }
            exact.push(home);
        }
        Self::new(&subtrees, &exact)
    }

    fn new(subtrees: &[PathBuf], exact: &[PathBuf]) -> Self {
        let keys = |v: &[PathBuf]| v.iter().filter_map(|p| canon(p).ok()).map(|c| c.key).collect();
        Self { subtrees: keys(subtrees), exact: keys(exact), allowed: Vec::new() }
    }

    /// Autorise le contenu de `dir` (pas `dir` lui-même) même s'il est dans un sous-arbre interdit.
    pub fn allow(mut self, dir: &Path) -> Self {
        if let Ok(c) = canon(dir) {
            self.allowed.push(c.key);
        }
        self
    }
}

/// Autorise l'envoi de `p` à la Corbeille sauf emplacement protégé ou fichier système.
pub fn check_trash(p: &Path, protected: &Protected) -> Result<PathBuf, GuardError> {
    check_lexical(p)?;
    // canonicalize suit les jonctions : sans ce refus, c'est leur cible qui partirait à la Corbeille.
    no_links(p)?;
    let c = canon(p)?;
    // Racine de disque : "\\?\c:\".
    if c.key.trim_end_matches('\\').ends_with(':') {
        return Err(GuardError::Protected);
    }
    let allowed = protected.allowed.iter().any(|a| inside(&c.key, a));
    if (!allowed && protected.subtrees.iter().any(|s| c.key == *s || inside(&c.key, s))) || protected.exact.contains(&c.key) {
        return Err(GuardError::Protected);
    }
    let m = std::fs::symlink_metadata(&c.path).map_err(|_| GuardError::NotFound)?;
    if m.file_attributes() & FILE_ATTRIBUTE_SYSTEM != 0 {
        return Err(GuardError::SystemFile);
    }
    Ok(c.path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    /// Dossier temporaire propre à un test, supprimé à la fin.
    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("kleaner-guard-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self, rel: &str) -> PathBuf {
            let p = self.0.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, b"x").unwrap();
            p
        }

        fn dir(&self, rel: &str) -> PathBuf {
            let p = self.0.join(rel);
            fs::create_dir_all(&p).unwrap();
            p
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            // Retire d'abord les jonctions (rmdir ne suit pas le lien).
            let _ = Command::new("cmd").args(["/C", "rmdir", "/S", "/Q"]).arg(&self.0).status();
        }
    }

    fn junction(link: &Path, target: &Path) {
        let ok = Command::new("cmd").args(["/C", "mklink", "/J"]).arg(link).arg(target).output().unwrap();
        assert!(ok.status.success(), "mklink /J a échoué");
    }

    #[test]
    fn delete_inside_root_ok() {
        let s = Sandbox::new("inside");
        let root = s.dir("cache");
        let f = s.file("cache/sub/a.tmp");
        assert!(check_delete(&f, &[root]).is_ok());
    }

    #[test]
    fn delete_root_itself_refused() {
        let s = Sandbox::new("rootself");
        let root = s.dir("cache");
        assert_eq!(check_delete(&root, std::slice::from_ref(&root)), Err(GuardError::OutsideRoots));
    }

    #[test]
    fn delete_outside_root_refused() {
        let s = Sandbox::new("outside");
        let root = s.dir("cache");
        let f = s.file("docs/precious.txt");
        assert_eq!(check_delete(&f, &[root]), Err(GuardError::OutsideRoots));
    }

    #[test]
    fn delete_sibling_with_same_prefix_refused() {
        let s = Sandbox::new("prefix");
        let root = s.dir("cache");
        let f = s.file("cache2/a.tmp");
        assert_eq!(check_delete(&f, &[root]), Err(GuardError::OutsideRoots));
    }

    #[test]
    fn delete_parent_dir_refused() {
        let s = Sandbox::new("dotdot");
        let root = s.dir("cache");
        s.file("docs/precious.txt");
        let sneaky = root.join("..").join("docs").join("precious.txt");
        assert_eq!(check_delete(&sneaky, &[root]), Err(GuardError::ParentDir));
    }

    #[test]
    fn delete_relative_refused() {
        assert_eq!(check_delete(Path::new("cache\\a.tmp"), &[]), Err(GuardError::Relative));
    }

    #[test]
    fn delete_unc_refused() {
        let p = Path::new(r"\\localhost\c$\Windows\Temp\a.tmp");
        assert_eq!(check_delete(p, &[PathBuf::from(r"\\localhost\c$\Windows\Temp")]), Err(GuardError::NotLocal));
    }

    #[test]
    fn delete_case_variant_ok() {
        let s = Sandbox::new("case");
        let root = s.dir("Cache");
        s.file("Cache/A.tmp");
        let upper = PathBuf::from(root.join("A.tmp").to_string_lossy().to_uppercase());
        assert!(check_delete(&upper, &[root]).is_ok());
    }

    #[test]
    fn delete_through_junction_refused() {
        let s = Sandbox::new("junction");
        let root = s.dir("cache");
        let docs = s.dir("docs");
        s.file("docs/precious.txt");
        let link = root.join("trap");
        junction(&link, &docs);
        assert_eq!(check_delete(&link.join("precious.txt"), std::slice::from_ref(&root)), Err(GuardError::Link));
        assert_eq!(check_delete(&link, &[root]), Err(GuardError::Link));
    }

    #[test]
    fn delete_root_behind_junction_refused() {
        let s = Sandbox::new("rootjunction");
        let real = s.dir("real");
        s.file("real/a.tmp");
        let link = s.0.join("cache");
        junction(&link, &real);
        assert_eq!(check_delete(&link.join("a.tmp"), std::slice::from_ref(&link)), Err(GuardError::Link));
    }

    #[test]
    fn delete_missing_refused() {
        let s = Sandbox::new("missing");
        let root = s.dir("cache");
        assert_eq!(check_delete(&root.join("nope.tmp"), &[root]), Err(GuardError::NotFound));
    }

    #[test]
    fn trash_user_file_ok() {
        let s = Sandbox::new("trashok");
        let f = s.file("big.iso");
        assert!(check_trash(&f, &Protected::from_env()).is_ok());
    }

    #[test]
    fn trash_drive_root_refused() {
        assert_eq!(check_trash(Path::new("C:\\"), &Protected::from_env()), Err(GuardError::Protected));
    }

    #[test]
    fn trash_windows_refused() {
        let win = PathBuf::from(std::env::var_os("SystemRoot").unwrap());
        let p = Protected::from_env();
        assert_eq!(check_trash(&win.join("System32"), &p), Err(GuardError::Protected));
        // "C:\Windows.\System32" : Windows ignore le point final.
        let dotted = PathBuf::from(format!("{}.", win.display())).join("System32");
        assert_eq!(check_trash(&dotted, &p), Err(GuardError::Protected));
    }

    #[test]
    fn trash_whole_user_folder_refused_but_content_ok() {
        let s = Sandbox::new("userfolder");
        let docs = s.dir("Documents");
        let f = s.file("Documents/a.txt");
        let p = Protected::new(&[], std::slice::from_ref(&docs));
        assert_eq!(check_trash(&docs, &p), Err(GuardError::Protected));
        assert!(check_trash(&f, &p).is_ok());
    }

    #[test]
    fn trash_through_junction_refused() {
        let s = Sandbox::new("trashjunction");
        let docs = s.dir("docs");
        s.file("docs/precious.txt");
        let link = s.0.join("leftover");
        junction(&link, &docs);
        let p = Protected::new(&[], &[]);
        assert_eq!(check_trash(&link, &p), Err(GuardError::Link), "la jonction elle-même");
        assert_eq!(check_trash(&link.join("precious.txt"), &p), Err(GuardError::Link), "à travers la jonction");
    }

    #[test]
    fn allowed_dir_opens_only_its_content() {
        let s = Sandbox::new("allowed");
        let pd = s.dir("ProgramData");
        let menu = s.dir("ProgramData/Start Menu");
        let lnk = s.file("ProgramData/Start Menu/app.lnk");
        let other = s.file("ProgramData/other.dat");
        let p = Protected::new(std::slice::from_ref(&pd), &[]).allow(&menu);
        assert!(check_trash(&lnk, &p).is_ok());
        assert_eq!(check_trash(&menu, &p), Err(GuardError::Protected), "le dossier lui-même reste protégé");
        assert_eq!(check_trash(&other, &p), Err(GuardError::Protected));
    }

    #[test]
    fn trash_system_file_refused() {
        let s = Sandbox::new("sysattr");
        let f = s.file("sys.bin");
        assert!(Command::new("attrib").arg("+s").arg(&f).status().unwrap().success());
        assert_eq!(check_trash(&f, &Protected::new(&[], &[])), Err(GuardError::SystemFile));
        let _ = Command::new("attrib").arg("-s").arg(&f).status();
    }

    /// Lien symbolique (dossier ou fichier) : il faut les droits admin ou le mode développeur pour en créer.
    /// Sans eux le test est sauté (le CI de GitHub tourne en admin et le fait).
    fn symlink(link: &Path, target: &Path, dir: bool) -> bool {
        let mut c = Command::new("cmd");
        c.args(["/C", "mklink"]);
        if dir {
            c.arg("/D");
        }
        let ok = c.arg(link).arg(target).output().is_ok_and(|o| o.status.success());
        if !ok {
            eprintln!("lien symbolique impossible ici (droits) : test sauté");
        }
        ok
    }

    #[test]
    fn symlinks_are_refused_like_junctions() {
        let s = Sandbox::new("symlink");
        let root = s.dir("cache");
        let docs = s.dir("docs");
        let precious = s.file("docs/precious.txt");
        let (dlink, flink) = (root.join("dirlink"), root.join("filelink.txt"));
        if !symlink(&dlink, &docs, true) || !symlink(&flink, &precious, false) {
            return;
        }
        let roots = std::slice::from_ref(&root);
        assert_eq!(check_delete(&dlink, roots), Err(GuardError::Link), "lien de dossier");
        assert_eq!(check_delete(&dlink.join("precious.txt"), roots), Err(GuardError::Link), "à travers le lien de dossier");
        assert_eq!(check_delete(&flink, roots), Err(GuardError::Link), "lien de fichier");
        let p = Protected::new(&[], &[]);
        assert_eq!(check_trash(&dlink, &p), Err(GuardError::Link));
        assert_eq!(check_trash(&flink, &p), Err(GuardError::Link));
        assert!(precious.is_file());
    }

    /// Chemins de périphérique, de volume ou de partage : jamais acceptés, même s'ils désignent un dossier autorisé.
    #[test]
    fn device_volume_and_network_forms_refused() {
        let s = Sandbox::new("device");
        let root = s.dir("cache");
        let f = s.file("cache/a.tmp");
        let plain = f.display().to_string();
        let roots = std::slice::from_ref(&root);
        for p in [
            format!(r"\\.\{plain}"),
            format!(r"\\?\GLOBALROOT\Device\HarddiskVolume1\{}", &plain[3..]),
            r"\\?\Volume{00000000-0000-0000-0000-000000000000}\a.tmp".to_string(),
            format!(r"\\?\UNC\localhost\{}$\{}", &plain[..1], &plain[3..]),
            format!(r"\\localhost\{}$\{}", &plain[..1], &plain[3..]),
        ] {
            assert_eq!(check_delete(Path::new(&p), roots), Err(GuardError::NotLocal), "{p}");
            assert_eq!(check_trash(Path::new(&p), &Protected::new(&[], &[])), Err(GuardError::NotLocal), "{p}");
        }
    }

    /// `\\?\C:\…` est accepté, mais Windows n'y interprète pas « .. » : impossible de sortir du dossier par là.
    #[test]
    fn verbatim_paths_cannot_climb_out() {
        let s = Sandbox::new("verbatim");
        let root = s.dir("cache");
        s.file("cache/a.tmp");
        s.file("docs/precious.txt");
        // Avec « \\?\ », « / » n'est pas un séparateur : chemin construit avec « \ ».
        let f = root.join("a.tmp");
        let roots = std::slice::from_ref(&root);
        assert!(check_delete(Path::new(&format!(r"\\?\{}", f.display())), roots).is_ok());
        let sneaky = format!(r"\\?\{}\..\docs\precious.txt", root.display());
        assert!(check_delete(Path::new(&sneaky), roots).is_err(), "{sneaky}");
    }

    /// Flux NTFS (« fichier:flux ») d'un fichier hors des racines : refusé comme le fichier lui-même.
    #[test]
    fn alternate_data_streams_follow_their_file() {
        let s = Sandbox::new("ads");
        let root = s.dir("cache");
        let outside = s.file("docs/precious.txt");
        let stream = PathBuf::from(format!("{}:secret", outside.display()));
        fs::write(&stream, b"hidden").unwrap();
        assert!(check_delete(&stream, std::slice::from_ref(&root)).is_err());
        assert_eq!(fs::read(&outside).unwrap(), b"x");
    }

    /// Lien physique dans un cache vers un fichier personnel : le supprimer n'efface que ce nom, le fichier reste.
    #[test]
    fn hard_link_deletion_keeps_the_other_name() {
        let s = Sandbox::new("hardlink");
        let root = s.dir("cache");
        let precious = s.file("docs/precious.txt");
        let link = root.join("same.txt");
        fs::hard_link(&precious, &link).unwrap();
        let ok = check_delete(&link, std::slice::from_ref(&root)).unwrap();
        fs::remove_file(ok).unwrap();
        assert_eq!(fs::read(&precious).unwrap(), b"x", "le fichier personnel est intact");
    }

    /// Windows ignore les espaces et points finaux : « C:\Windows \System32 » reste protégé.
    #[test]
    fn trailing_space_or_dot_does_not_escape_protection() {
        let win = PathBuf::from(std::env::var_os("SystemRoot").unwrap());
        let p = Protected::from_env();
        for v in [format!("{} ", win.display()), format!("{}. ", win.display()), format!("{}...", win.display())] {
            let path = PathBuf::from(v).join("System32");
            assert!(check_trash(&path, &p).is_err(), "{}", path.display());
        }
    }

    /// Nom court 8.3 (« PROGRA~1 ») : résolu en nom long avant la comparaison.
    #[test]
    fn short_names_are_resolved() {
        let s = Sandbox::new("short");
        let root = s.dir("cache with long name");
        let f = s.file("cache with long name/a.tmp");
        let out = Command::new("cmd").args(["/C", "for", "%I", "in"]).arg(format!("(\"{}\")", root.display())).args(["do", "@echo", "%~sI"]).output().unwrap();
        let short = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if short.is_empty() || short.eq_ignore_ascii_case(&root.display().to_string()) {
            eprintln!("noms courts désactivés sur ce disque : test sauté");
            return;
        }
        assert_eq!(check_delete(&PathBuf::from(&short).join("a.tmp"), std::slice::from_ref(&root)).unwrap(), fs::canonicalize(&f).unwrap());
        assert!(check_delete(&root.join("a.tmp"), &[PathBuf::from(&short)]).is_ok(), "racine donnée en nom court");
    }

    #[test]
    fn delete_inside_removes_only_what_it_should() {
        let s = Sandbox::new("delinside");
        let root = s.dir("cache");
        let roots = std::slice::from_ref(&root);
        let f = s.file("cache/a.tmp");
        let ro = s.file("cache/ro.tmp");
        let outside = s.file("docs/precious.txt");
        let full = s.dir("cache/full");
        s.file("cache/full/x.tmp");
        let empty = s.dir("cache/empty");
        assert!(Command::new("attrib").arg("+r").arg(&ro).status().unwrap().success());

        assert!(delete_inside(&f, roots, false) && !f.exists(), "fichier du cache");
        assert!(delete_inside(&ro, roots, false) && !ro.exists(), "lecture seule");
        assert!(!delete_inside(&outside, roots, false) && outside.exists(), "hors du cache");
        assert!(!delete_inside(&full, roots, true) && full.exists(), "dossier non vide");
        assert!(!delete_inside(&empty, roots, false) && empty.exists(), "un dossier n'est pas un fichier");
        assert!(!delete_inside(&full.join("x.tmp"), roots, true), "un fichier n'est pas un dossier");
        assert!(delete_inside(&empty, roots, true) && !empty.exists(), "dossier vide");
        assert!(!delete_inside(&root, roots, true) && root.exists(), "jamais la racine elle-même");
    }

    /// État du disque après une substitution : un dossier parent est devenu une jonction vers un dossier personnel.
    /// Le texte du chemin est encore « dans le cache », mais le fichier ouvert est ailleurs : refusé.
    #[test]
    fn swapped_parent_is_caught_on_the_opened_file() {
        let s = Sandbox::new("swap");
        let root = s.dir("cache");
        let docs = s.dir("docs");
        let precious = s.file("docs/precious.txt");
        junction(&root.join("sub"), &docs);
        let roots = std::slice::from_ref(&root);
        let key = canon(&root).unwrap().key;
        let via = root.join("sub").join("precious.txt");
        assert!(open_checked(&via, &key, false).is_none(), "le vrai chemin du fichier ouvert est hors du cache");
        assert!(!delete_inside(&via, roots, false));
        assert!(precious.is_file());
        // Sans jonction, le même contrôle laisse passer un fichier du cache.
        let ok = s.file("cache/b.tmp");
        assert!(open_checked(&ok, &key, false).is_some());
    }

    /// Secours des anciens Windows 10 (pas de suppression immédiate) : fichier normal et lecture seule.
    #[test]
    fn classic_deletion_fallback_works() {
        let s = Sandbox::new("classic");
        let root = s.dir("cache");
        let roots = std::slice::from_ref(&root);
        let (f, ro) = (s.file("cache/a.tmp"), s.file("cache/ro.tmp"));
        assert!(Command::new("attrib").arg("+r").arg(&ro).status().unwrap().success());
        for p in [&f, &ro] {
            let h = open_inside(p, roots, false).unwrap();
            assert!(dispose_classic(&h, p), "{}", p.display());
            drop(h); // effective à la fermeture
            assert!(!p.exists(), "{}", p.display());
        }
    }

    /// Fichier ouvert par un programme sans partage de suppression : refusé, rien ne bouge.
    #[test]
    fn locked_file_is_skipped() {
        let s = Sandbox::new("locked");
        let root = s.dir("cache");
        let f = s.file("cache/busy.tmp");
        use std::os::windows::fs::OpenOptionsExt;
        let _open = fs::OpenOptions::new().read(true).share_mode(0).open(&f).unwrap();
        assert!(!delete_inside(&f, std::slice::from_ref(&root), false));
        drop(_open);
        assert!(f.exists());
    }

    #[test]
    fn inside_is_strict() {
        assert!(inside(r"\\?\c:\a\b", r"\\?\c:\a"));
        assert!(!inside(r"\\?\c:\a", r"\\?\c:\a"));
        assert!(!inside(r"\\?\c:\ab", r"\\?\c:\a"));
    }
}
