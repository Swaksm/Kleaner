//! Déplacer un gros dossier personnel vers un autre disque (Analyse → Déplacer…), pour libérer le disque système.
//! L'original est d'abord renommé (Windows refuse si un fichier dedans est ouvert), puis copié fichier par fichier ;
//! chaque copie est relue et son empreinte blake3 comparée à celle de l'original. L'original n'est retiré que quand
//! tout est vérifié. Au moindre problème, la copie partielle (créée par Kleaner) est retirée et l'original reprend
//! son nom. Annulable depuis le Journal : le même chemin en sens inverse.

use crate::actions::ActionReport;
use crate::programs::Program;
use crate::scan::{self, DriveKind};
use crate::{guard, journal};
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

/// Dossier créé à la racine du disque choisi : `D:\Kleaner moved\Videos\Rushes`.
pub const FOLDER: &str = "Kleaner moved";
/// Place laissée libre sur le disque de destination en plus du dossier.
const MARGIN: u64 = 512 * 1024 * 1024;

const SYSTEM: u32 = 0x4;
const REPARSE_POINT: u32 = 0x400;
const OFFLINE: u32 = 0x1000;
const RECALL_ON_OPEN: u32 = 0x40000;
const RECALL_ON_DATA_ACCESS: u32 = 0x400000;

#[derive(Serialize, Debug)]
pub struct Target {
    /// Racine du disque (« D:\ »).
    pub root: String,
    /// Où le dossier arrivera.
    pub dest: String,
    pub free: u64,
}

#[derive(Serialize, Debug)]
pub struct Plan {
    pub source: String,
    pub files: u64,
    pub bytes: u64,
    /// Autres disques locaux (internes ou USB) ; place suffisante vérifiée au moment du déplacement.
    pub targets: Vec<Target>,
}

pub struct Item {
    rel: PathBuf,
    size: u64,
    dir: bool,
}

/// « \\?\C:\Users\… » → « C:\Users\… » (journal et affichage).
fn plain(p: &Path) -> String {
    p.display().to_string().trim_start_matches(r"\\?\").to_string()
}

fn drive_letter(p: &str) -> Option<char> {
    p.trim_start_matches(r"\\?\").chars().next().map(|c| c.to_ascii_uppercase())
}

/// Contenu du dossier, parents avant enfants, sans jamais suivre un lien.
/// Refus : lien ou jonction, fichier système, fichier en ligne seulement (OneDrive, il faudrait le télécharger).
fn walk(root: &Path) -> Result<Vec<Item>, String> {
    let mut out = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        let rd = fs::read_dir(root.join(&rel)).map_err(|e| format!("{} : {e}", plain(&root.join(&rel))))?;
        for e in rd {
            let e = e.map_err(|e| e.to_string())?;
            // Sous Windows, les métadonnées d'une entrée de dossier ne suivent pas les liens.
            let m = e.metadata().map_err(|err| format!("{} : {err}", plain(&e.path())))?;
            let a = m.file_attributes();
            if a & REPARSE_POINT != 0 {
                return Err(format!("Refused: {} (link or junction on the path)", plain(&e.path())));
            }
            if a & (OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS) != 0 {
                return Err(format!("Refused: {} (online-only file)", plain(&e.path())));
            }
            if a & SYSTEM != 0 {
                return Err(format!("Refused: {} (system file)", plain(&e.path())));
            }
            let child = rel.join(e.file_name());
            if m.is_dir() {
                stack.push(child.clone());
            }
            out.push(Item { rel: child, size: if m.is_dir() { 0 } else { m.len() }, dir: m.is_dir() });
        }
    }
    Ok(out)
}

/// Les dossiers personnels où Kleaner accepte de déplacer un sous-dossier.
pub fn personal_roots(home: &Path) -> Vec<PathBuf> {
    ["Desktop", "Documents", "Downloads", "Pictures", "Music", "Videos"].iter().map(|d| home.join(d)).collect()
}

/// Vérifie la source et la lit. Renvoie son chemin canonique et son contenu.
fn inspect(source: &Path, home: &Path, programs: &[Program]) -> Result<(PathBuf, Vec<Item>), String> {
    let c = guard::check_move(source, &personal_roots(home)).map_err(|e| format!("Refused: {} ({e})", source.display()))?;
    let key = plain(&c).to_lowercase();
    if programs.iter().any(|p| {
        let l = p.location.to_lowercase();
        !l.is_empty() && (l == key || l.starts_with(&format!("{key}\\")))
    }) {
        return Err(format!("Refused: {} (a program is installed there)", plain(&c)));
    }
    let items = walk(&c)?;
    Ok((c, items))
}

/// Ce que le déplacement ferait : taille, nombre de fichiers, disques possibles. Lecture seule.
pub fn plan(source: &Path, home: &Path, programs: &[Program]) -> Result<Plan, String> {
    let (c, items) = inspect(source, home, programs)?;
    let rel = c
        .strip_prefix(fs::canonicalize(home).map_err(|e| e.to_string())?)
        .map_err(|_| format!("Refused: {} (outside allowed locations)", plain(&c)))?
        .to_path_buf();
    let from = drive_letter(&plain(&c));
    let targets = scan::drives()
        .into_iter()
        .filter(|d| matches!(d.kind, DriveKind::Fixed | DriveKind::Removable) && drive_letter(&d.root) != from)
        .map(|d| Target { dest: plain(&Path::new(&d.root).join(FOLDER).join(&rel)), root: d.root, free: d.free })
        .collect();
    Ok(Plan {
        source: plain(&c),
        files: items.iter().filter(|i| !i.dir).count() as u64,
        bytes: items.iter().map(|i| i.size).sum(),
        targets,
    })
}

/// Déplace `source` vers le disque `root` (un des `targets` du plan, recalculé ici).
pub fn move_folder(
    source: &Path,
    root: &str,
    home: &Path,
    programs: &[Program],
    journal_dir: &Path,
    cancel: &AtomicBool,
    progress: &dyn Fn(u64, u64),
) -> Result<(ActionReport, String), String> {
    let p = plan(source, home, programs)?;
    let t = p.targets.iter().find(|t| t.root.eq_ignore_ascii_case(root)).ok_or(format!("Refused: {root} (not another local drive)"))?;
    if t.free < p.bytes + MARGIN {
        return Err(format!("Refused: not enough free space on {}", t.root));
    }
    let (from, items) = inspect(source, home, programs)?;
    let to = PathBuf::from(&t.dest);
    let report = transfer(&from, &to, &items, "move", journal_dir, cancel, progress)?;
    Ok((report, t.dest.clone()))
}

/// Remet un dossier déplacé par Kleaner à sa place d'origine (Journal → Restaurer).
/// `moved` doit être dans « <disque>:\Kleaner moved » et `original` ne doit plus exister.
pub fn move_back(original: &Path, moved: &Path, journal_dir: &Path, cancel: &AtomicBool, progress: &dyn Fn(u64, u64)) -> Result<ActionReport, String> {
    let drive = drive_letter(&moved.display().to_string()).map(|c| PathBuf::from(format!("{c}:\\"))).ok_or("Invalid path")?;
    let from = guard::check_move(moved, &[drive.join(FOLDER)]).map_err(|e| format!("Refused: {} ({e})", moved.display()))?;
    if fs::symlink_metadata(original).is_ok() {
        return Err(format!("Refused: {} already exists", original.display()));
    }
    guard::no_links(original).map_err(|e| format!("Refused: {} ({e})", original.display()))?;
    let items = walk(&from)?;
    let bytes: u64 = items.iter().map(|i| i.size).sum();
    let back = drive_letter(&original.display().to_string());
    if let Some(d) = scan::drives().into_iter().find(|d| drive_letter(&d.root) == back)
        && d.free < bytes + MARGIN
    {
        return Err(format!("Refused: not enough free space on {}", d.root));
    }
    let report = transfer(&from, original, &items, "move-back", journal_dir, cancel, progress)?;
    // Dossiers « Kleaner moved\Videos » devenus vides : retirés (remove_dir refuse un dossier non vide).
    for parent in from.ancestors().skip(1) {
        if fs::remove_dir(parent).is_err() || parent.file_name().is_some_and(|n| n == FOLDER) {
            break;
        }
    }
    Ok(report)
}

/// Copie vérifiée de `from` vers `to` (qui ne doit pas exister), puis retrait de `from`. Journalisé.
fn transfer(
    from: &Path,
    to: &Path,
    items: &[Item],
    action: &str,
    journal_dir: &Path,
    cancel: &AtomicBool,
    progress: &dyn Fn(u64, u64),
) -> Result<ActionReport, String> {
    let name = from.file_name().ok_or("Invalid path")?.to_string_lossy().to_string();
    let parked = from.with_file_name(format!("{name}.kleaner-moving"));
    if fs::symlink_metadata(&parked).is_ok() {
        return Err(format!("Refused: {} already exists", plain(&parked)));
    }
    guard::no_links(to).map_err(|e| format!("Refused: {} ({e})", to.display()))?;
    fs::create_dir_all(to.parent().ok_or("Invalid path")?).map_err(|e| format!("{} : {e}", to.display()))?;
    // create_dir (pas _all) : échoue si la destination existe, donc on ne retire jamais que ce qu'on a créé.
    fs::create_dir(to).map_err(|_| format!("Refused: {} already exists", to.display()))?;
    // Renommer d'abord : refusé si un fichier du dossier est ouvert, et plus rien n'y écrit pendant la copie.
    if fs::rename(from, &parked).is_err() {
        let _ = fs::remove_dir(to);
        return Err(format!("Refused: {} is in use, close the programs that use it", plain(from)));
    }

    let mut entry = journal::Entry::new(action);
    entry.paths = vec![plain(from), plain(to)];
    entry.files = items.iter().filter(|i| !i.dir).count() as u64;
    entry.bytes = items.iter().map(|i| i.size).sum();
    let result = match copy_verified(&parked, to, items, entry.bytes, cancel, progress) {
        Ok(()) => fs::remove_dir_all(&parked).map_err(|e| {
            format!("Copied and checked to {}, but the original could not be fully removed ({}): {e}", plain(to), plain(&parked))
        }),
        Err(e) => {
            let _ = fs::remove_dir_all(to);
            match fs::rename(&parked, from) {
                Ok(()) => Err(e),
                Err(r) => Err(format!("{e}. The original is intact but kept the name {} ({r})", plain(&parked))),
            }
        }
    };
    entry.error = result.as_ref().err().cloned();
    journal::append(journal_dir, &entry).map_err(|e| format!("Log: {e}"))?;
    result.map(|_| ActionReport { files: entry.files, bytes: entry.bytes })
}

fn copy_verified(from: &Path, to: &Path, items: &[Item], total: u64, cancel: &AtomicBool, progress: &dyn Fn(u64, u64)) -> Result<(), String> {
    let mut buf = vec![0u8; 1 << 20];
    let mut done = 0u64;
    let err = |p: &Path, e: std::io::Error| format!("{} : {e}", plain(p));
    for i in items {
        if cancel.load(Relaxed) {
            return Err("Cancelled".into());
        }
        let (src, dst) = (from.join(&i.rel), to.join(&i.rel));
        if i.dir {
            fs::create_dir(&dst).map_err(|e| err(&dst, e))?;
            continue;
        }
        let mut r = File::open(&src).map_err(|e| err(&src, e))?;
        let mut w = OpenOptions::new().write(true).create_new(true).open(&dst).map_err(|e| err(&dst, e))?;
        let mut sum = blake3::Hasher::new();
        let mut chunks = 0u32;
        loop {
            let n = r.read(&mut buf).map_err(|e| err(&src, e))?;
            if n == 0 {
                break;
            }
            sum.update(&buf[..n]);
            w.write_all(&buf[..n]).map_err(|e| err(&dst, e))?;
            done += n as u64;
            chunks += 1;
            if chunks.is_multiple_of(64) {
                if cancel.load(Relaxed) {
                    return Err("Cancelled".into());
                }
                progress(done, total);
            }
        }
        let m = r.metadata().map_err(|e| err(&src, e))?;
        if let Ok(t) = m.modified() {
            let _ = w.set_modified(t);
        }
        w.sync_all().map_err(|e| err(&dst, e))?;
        drop(w);
        // Relecture de la copie : même empreinte que l'original, sinon on arrête tout.
        let mut check = blake3::Hasher::new();
        check.update_reader(File::open(&dst).map_err(|e| err(&dst, e))?).map_err(|e| err(&dst, e))?;
        if check.finalize() != sum.finalize() {
            return Err(format!("Copy check failed: {}", plain(&src)));
        }
        if m.permissions().readonly() {
            let mut perm = m.permissions();
            perm.set_readonly(true);
            let _ = fs::set_permissions(&dst, perm);
        }
        progress(done, total);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("kleaner-mover-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fill(dir: &Path) {
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::write(dir.join("one.txt"), b"hello").unwrap();
        fs::write(dir.join("a/two.bin"), vec![7u8; 3 << 20]).unwrap();
        fs::write(dir.join("a/b/three"), b"").unwrap();
    }

    #[test]
    fn move_and_back_keeps_every_byte() {
        let s = Sandbox::new("roundtrip");
        let (src, dst, log) = (s.0.join("Rushes"), s.0.join("D/Kleaner moved/Videos/Rushes"), s.0.join("log"));
        fill(&src);
        let items = walk(&src).unwrap();
        assert_eq!(items.iter().filter(|i| !i.dir).count(), 3);
        let stop = AtomicBool::new(false);
        let r = transfer(&src, &dst, &items, "move", &log, &stop, &|_, _| {}).unwrap();
        assert_eq!((r.files, r.bytes), (3, 5 + (3 << 20)));
        assert!(!src.exists(), "original retiré");
        assert_eq!(fs::read(dst.join("a/two.bin")).unwrap(), vec![7u8; 3 << 20]);
        let entries = journal::read_last(&log, 10).unwrap();
        assert_eq!(entries[0].action, "move");
        assert_eq!(entries[0].paths, [plain(&src), plain(&dst)]);

        let back = walk(&dst).unwrap();
        transfer(&dst, &src, &back, "move-back", &log, &stop, &|_, _| {}).unwrap();
        assert_eq!(fs::read(src.join("one.txt")).unwrap(), b"hello");
        assert!(src.join("a/b/three").is_file() && !dst.exists());
    }

    #[test]
    fn cancel_leaves_the_original_untouched() {
        let s = Sandbox::new("cancel");
        let (src, dst, log) = (s.0.join("Big"), s.0.join("D/Big"), s.0.join("log"));
        fill(&src);
        let items = walk(&src).unwrap();
        let stop = AtomicBool::new(true);
        let e = transfer(&src, &dst, &items, "move", &log, &stop, &|_, _| {}).unwrap_err();
        assert_eq!(e, "Cancelled");
        assert!(src.join("a/two.bin").is_file() && !dst.exists(), "original à sa place, copie partielle retirée");
        assert!(journal::read_last(&log, 1).unwrap()[0].error.is_some());
    }

    #[test]
    fn existing_destination_is_never_merged() {
        let s = Sandbox::new("exists");
        let (src, dst, log) = (s.0.join("Src"), s.0.join("Dst"), s.0.join("log"));
        fill(&src);
        fs::create_dir_all(&dst).unwrap();
        fs::write(dst.join("mine"), b"keep").unwrap();
        let items = walk(&src).unwrap();
        assert!(transfer(&src, &dst, &items, "move", &log, &AtomicBool::new(false), &|_, _| {}).unwrap_err().contains("already exists"));
        assert_eq!(fs::read(dst.join("mine")).unwrap(), b"keep");
        assert!(src.join("one.txt").is_file());
    }

    #[test]
    fn open_file_blocks_the_move() {
        let s = Sandbox::new("locked");
        let (src, dst, log) = (s.0.join("Busy"), s.0.join("D/Busy"), s.0.join("log"));
        fill(&src);
        let items = walk(&src).unwrap();
        let _open = File::open(src.join("one.txt")).unwrap();
        let e = transfer(&src, &dst, &items, "move", &log, &AtomicBool::new(false), &|_, _| {}).unwrap_err();
        assert!(e.contains("in use"), "{e}");
        assert!(src.join("one.txt").is_file() && !dst.exists());
    }

    #[test]
    fn junction_inside_is_refused() {
        let s = Sandbox::new("junction");
        let src = s.0.join("WithLink");
        fill(&src);
        let target = s.0.join("elsewhere");
        fs::create_dir_all(&target).unwrap();
        let ok = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(src.join("link"))
            .arg(&target)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok);
        assert!(walk(&src).err().unwrap().contains("link or junction"));
    }

    /// Sur la vraie machine, avec un deuxième disque : un petit dossier de test dans Vidéos part et revient.
    /// `cargo test --release mover::tests::real_round_trip -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_round_trip() {
        let home = PathBuf::from(std::env::var_os("USERPROFILE").unwrap());
        let src = home.join("Videos").join("kleaner-move-test");
        let _ = fs::remove_dir_all(&src);
        fill(&src);
        let log = std::env::temp_dir().join("kleaner-move-real-log");
        let p = plan(&src, &home, &[]).unwrap();
        println!("{} fichiers, {} octets, disques : {:?}", p.files, p.bytes, p.targets);
        let t = p.targets.first().expect("un deuxième disque est nécessaire");
        let stop = AtomicBool::new(false);
        let (r, dest) = move_folder(&src, &t.root, &home, &[], &log, &stop, &|_, _| {}).unwrap();
        println!("déplacé : {dest} ({} fichiers)", r.files);
        assert!(!src.exists() && Path::new(&dest).join("a/two.bin").is_file());
        move_back(&src, Path::new(&dest), &log, &stop, &|_, _| {}).unwrap();
        assert!(src.join("a/two.bin").is_file() && !Path::new(&dest).exists());
        assert!(!Path::new(&t.root).join(FOLDER).exists(), "dossiers vides retirés");
        fs::remove_dir_all(&src).unwrap();
        let _ = fs::remove_dir_all(&log);
    }

    #[test]
    fn only_inside_personal_folders() {
        let s = Sandbox::new("personal");
        let home = s.0.join("home");
        fill(&home.join("Videos/Rushes"));
        fill(&home.join("AppData/Local/App"));
        let installed = Program {
            id: "x".into(),
            name: "Game".into(),
            publisher: String::new(),
            version: String::new(),
            bytes: 0,
            install_date: String::new(),
            location: plain(&fs::canonicalize(home.join("Videos/Rushes")).unwrap()) + "\\a",
        };
        let p = plan(&home.join("Videos/Rushes"), &home, &[]).unwrap();
        assert_eq!((p.files, p.bytes), (3, 5 + (3 << 20)));
        assert!(p.targets.iter().all(|t| t.dest.ends_with(r"Kleaner moved\Videos\Rushes")));
        assert!(plan(&home.join("Videos"), &home, &[]).is_err(), "le dossier Vidéos lui-même : non");
        assert!(plan(&home.join("AppData/Local/App"), &home, &[]).is_err(), "AppData : non");
        assert!(plan(&home.join("Videos/Rushes"), &home, &[installed]).unwrap_err().contains("program is installed"));
    }
}
