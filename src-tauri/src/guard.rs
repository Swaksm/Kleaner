//! Garde de sécurité : toute suppression ou mise à la Corbeille passe par ici.
//!
//! - `check_delete` : suppression définitive, autorisée uniquement *à l'intérieur*
//!   des racines déclarées dans le catalogue (allowlist).
//! - `check_trash` : mise à la Corbeille d'un fichier choisi par l'utilisateur,
//!   refusée dans les emplacements système (blocklist).

use std::fmt;
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
fn no_links(p: &Path) -> Result<(), GuardError> {
    for a in p.ancestors() {
        if let Ok(m) = std::fs::symlink_metadata(a)
            && m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        {
            return Err(GuardError::Link);
        }
    }
    Ok(())
}

/// Autorise la suppression définitive de `p` seulement s'il est strictement sous une des `roots`.
pub fn check_delete(p: &Path, roots: &[PathBuf]) -> Result<PathBuf, GuardError> {
    check_lexical(p)?;
    no_links(p)?;
    let c = canon(p)?;
    for root in roots {
        if let Ok(r) = canon(root)
            && inside(&c.key, &r.key)
        {
            return Ok(c.path);
        }
    }
    Err(GuardError::OutsideRoots)
}

/// Emplacements interdits à la mise à la Corbeille.
pub struct Protected {
    /// Interdits, eux et tout ce qu'ils contiennent.
    subtrees: Vec<String>,
    /// Interdits en tant que dossier entier, leur contenu reste autorisé.
    exact: Vec<String>,
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
        Self { subtrees: keys(subtrees), exact: keys(exact) }
    }
}

/// Autorise l'envoi de `p` à la Corbeille sauf emplacement protégé ou fichier système.
pub fn check_trash(p: &Path, protected: &Protected) -> Result<PathBuf, GuardError> {
    let c = canon(p)?;
    // Racine de disque : "\\?\c:\".
    if c.key.trim_end_matches('\\').ends_with(':') {
        return Err(GuardError::Protected);
    }
    if protected.subtrees.iter().any(|s| c.key == *s || inside(&c.key, s)) || protected.exact.contains(&c.key) {
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
    fn trash_system_file_refused() {
        let s = Sandbox::new("sysattr");
        let f = s.file("sys.bin");
        assert!(Command::new("attrib").arg("+s").arg(&f).status().unwrap().success());
        assert_eq!(check_trash(&f, &Protected::new(&[], &[])), Err(GuardError::SystemFile));
        let _ = Command::new("attrib").arg("-s").arg(&f).status();
    }

    #[test]
    fn inside_is_strict() {
        assert!(inside(r"\\?\c:\a\b", r"\\?\c:\a"));
        assert!(!inside(r"\\?\c:\a", r"\\?\c:\a"));
        assert!(!inside(r"\\?\c:\ab", r"\\?\c:\a"));
    }
}
