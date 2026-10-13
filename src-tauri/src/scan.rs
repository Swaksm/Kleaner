//! Analyse des disques : parcours parallèle, fichiers cachés compris, liens et jonctions ignorés.
//! Lecture seule : rien n'est modifié.

use rayon::prelude::*;
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::time::Instant;

const TOP_N: usize = 200;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
/// Fichiers « en ligne uniquement » (OneDrive…) : ils n'occupent pas le disque.
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
/// Un dossier dont un seul enfant porte ≥ 80 % de la taille n'est qu'un passage : on ne le liste pas.
const PASS_THROUGH: f64 = 0.8;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Videos,
    Images,
    Audio,
    Archives,
    Installers,
    Other,
}

const CATEGORIES: [Category; 6] = [
    Category::Videos,
    Category::Images,
    Category::Audio,
    Category::Archives,
    Category::Installers,
    Category::Other,
];

fn category(path: &Path) -> Category {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "webm" | "m4v" | "flv" | "mpg" | "mpeg" => Category::Videos,
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "tif" | "tiff" | "psd" | "raw" | "cr2" | "nef" => {
            Category::Images
        }
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" => Category::Audio,
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso" | "img" => Category::Archives,
        // .exe exclu : impossible de distinguer un installeur d'un programme installé.
        "msi" | "msix" | "msixbundle" | "appx" | "appxbundle" => Category::Installers,
        _ => Category::Other,
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Item {
    pub size: u64,
    pub path: String,
    /// Dernière modification (secondes Unix). Pour un dossier : le fichier le plus récent dedans.
    pub modified: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct CategoryTotal {
    pub category: Category,
    pub bytes: u64,
    pub files: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct ScanResult {
    pub roots: Vec<String>,
    pub bytes: u64,
    pub files: u64,
    /// Dossiers illisibles (accès refusé), ignorés.
    pub denied: u64,
    pub cancelled: bool,
    pub duration_ms: u64,
    pub top_files: Vec<Item>,
    pub top_dirs: Vec<Item>,
    pub categories: Vec<CategoryTotal>,
}

/// Compteurs lus par le fil de progression pendant le scan.
#[derive(Default)]
pub struct Progress {
    pub files: AtomicU64,
    pub bytes: AtomicU64,
    pub denied: AtomicU64,
}

/// Garde les N plus gros éléments ; le seuil atomique évite de verrouiller pour les petits.
struct TopN {
    heap: Mutex<BinaryHeap<Reverse<Item>>>,
    min: AtomicU64,
}

impl TopN {
    fn new() -> Self {
        Self { heap: Mutex::new(BinaryHeap::with_capacity(TOP_N + 1)), min: AtomicU64::new(0) }
    }

    fn offer(&self, size: u64, path: &Path, modified: u64) {
        if size <= self.min.load(Relaxed) {
            return;
        }
        let mut heap = self.heap.lock().unwrap();
        heap.push(Reverse(Item { size, path: path.display().to_string(), modified }));
        if heap.len() > TOP_N {
            heap.pop();
            self.min.store(heap.peek().map_or(0, |r| r.0.size), Relaxed);
        }
    }

    fn into_sorted(self) -> Vec<Item> {
        let mut v: Vec<Item> = self.heap.into_inner().unwrap().into_iter().map(|r| r.0).collect();
        v.sort_by(|a, b| b.cmp(a));
        v
    }
}

struct Ctx<'a> {
    cancel: &'a AtomicBool,
    progress: &'a Progress,
    files: TopN,
    dirs: TopN,
    cats: [(AtomicU64, AtomicU64); 6],
}

fn unix_secs(t: std::io::Result<std::time::SystemTime>) -> u64 {
    t.ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs())
}

/// (taille totale, modification la plus récente) de `dir`, en enregistrant fichiers et dossiers au passage.
// shortcut: les liens physiques (WinSxS ↔ System32) sont comptés deux fois ; à corriger si les tailles de C:\Windows gênent.
fn walk(dir: &Path, ctx: &Ctx) -> (u64, u64) {
    if ctx.cancel.load(Relaxed) {
        return (0, 0);
    }
    let Ok(rd) = fs::read_dir(dir) else {
        ctx.progress.denied.fetch_add(1, Relaxed);
        return (0, 0);
    };
    let mut own = 0u64;
    let mut newest = 0u64;
    let mut subdirs = Vec::new();
    for entry in rd.flatten() {
        // Sous Windows, ces métadonnées viennent de la lecture du dossier : pas d'appel système en plus.
        let Ok(m) = entry.metadata() else { continue };
        let attrs = m.file_attributes();
        if attrs & FILE_ATTRIBUTE_REPARSE_POINT != 0 && m.is_dir() {
            continue; // jonction ou lien : déjà compté ailleurs, ou hors disque
        }
        if m.is_dir() {
            subdirs.push(entry.path());
            continue;
        }
        if attrs & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS != 0 {
            continue;
        }
        let len = m.len();
        own += len;
        let modified = unix_secs(m.modified());
        newest = newest.max(modified);
        let path = entry.path();
        let c = category(&path) as usize;
        ctx.cats[c].0.fetch_add(len, Relaxed);
        ctx.cats[c].1.fetch_add(1, Relaxed);
        ctx.progress.files.fetch_add(1, Relaxed);
        ctx.progress.bytes.fetch_add(len, Relaxed);
        ctx.files.offer(len, &path, modified);
    }
    let children: Vec<(u64, u64)> = subdirs.par_iter().map(|d| walk(d, ctx)).collect();
    let total = own + children.iter().map(|c| c.0).sum::<u64>();
    let newest = children.iter().map(|c| c.1).fold(newest, u64::max);
    let biggest_child = children.iter().map(|c| c.0).max().unwrap_or(0);
    if total > 0 && (biggest_child as f64) < PASS_THROUGH * total as f64 {
        ctx.dirs.offer(total, dir, newest);
    }
    (total, newest)
}

pub fn scan(roots: &[PathBuf], cancel: &AtomicBool, progress: &Progress) -> ScanResult {
    let start = Instant::now();
    let ctx = Ctx {
        cancel,
        progress,
        files: TopN::new(),
        dirs: TopN::new(),
        cats: Default::default(),
    };
    let bytes: u64 = roots.par_iter().map(|r| walk(r, &ctx).0).sum();
    // Une racine de disque dans le top n'apprend rien à l'utilisateur.
    let roots_str: Vec<String> = roots.iter().map(|r| r.display().to_string()).collect();
    let mut top_dirs = ctx.dirs.into_sorted();
    top_dirs.retain(|i| !roots_str.contains(&i.path));
    let categories = CATEGORIES
        .iter()
        .zip(&ctx.cats)
        .map(|(&category, (b, f))| CategoryTotal { category, bytes: b.load(Relaxed), files: f.load(Relaxed) })
        .collect();
    ScanResult {
        roots: roots_str,
        bytes,
        files: progress.files.load(Relaxed),
        denied: progress.denied.load(Relaxed),
        cancelled: cancel.load(Relaxed),
        duration_ms: start.elapsed().as_millis() as u64,
        top_files: ctx.files.into_sorted(),
        top_dirs,
        categories,
    }
}

/// Disques fixes (pas les clés USB, lecteurs réseau ni CD).
pub fn fixed_drives() -> Vec<PathBuf> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    const DRIVE_FIXED: u32 = 3;
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| format!("{}:\\", (b'A' + i) as char))
        .filter(|root| {
            let wide: Vec<u16> = root.encode_utf16().chain([0]).collect();
            unsafe { GetDriveTypeW(wide.as_ptr()) == DRIVE_FIXED }
        })
        .map(PathBuf::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("kleaner-scan-{}-{name}", std::process::id()));
            let _ = Command::new("cmd").args(["/C", "rmdir", "/S", "/Q"]).arg(&dir).status();
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self, rel: &str, size: usize) -> PathBuf {
            let p = self.0.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, vec![0u8; size]).unwrap();
            p
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = Command::new("cmd").args(["/C", "rmdir", "/S", "/Q"]).arg(&self.0).status();
        }
    }

    fn run(root: &Path) -> ScanResult {
        scan(&[root.to_path_buf()], &AtomicBool::new(false), &Progress::default())
    }

    #[test]
    fn sizes_sorting_and_categories() {
        let s = Sandbox::new("sizes");
        s.file("a/film.mkv", 3000);
        s.file("a/photo.JPG", 2000);
        s.file("b/notes.txt", 1000);
        s.file("b/setup.msi", 500);
        let r = run(&s.0);
        assert_eq!(r.bytes, 6500);
        assert_eq!(r.files, 4);
        let sizes: Vec<u64> = r.top_files.iter().map(|i| i.size).collect();
        assert_eq!(sizes, [3000, 2000, 1000, 500]);
        assert!(r.top_files[0].path.ends_with("film.mkv"));
        let cat = |c| r.categories.iter().find(|t| t.category == c).unwrap().bytes;
        assert_eq!(cat(Category::Videos), 3000);
        assert_eq!(cat(Category::Images), 2000);
        assert_eq!(cat(Category::Installers), 500);
        assert_eq!(cat(Category::Other), 1000);
    }

    #[test]
    fn dir_modified_is_newest_file_inside() {
        let s = Sandbox::new("modified");
        let old = s.file("proj/old.bin", 3000);
        let new = s.file("proj/sub/new.bin", 10);
        let set = |p: &Path, secs: u64| {
            let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs);
            fs::File::options().write(true).open(p).unwrap().set_modified(t).unwrap();
        };
        set(&old, 1_000_000_000);
        set(&new, 1_600_000_000);
        let r = run(&s.0);
        let file = r.top_files.iter().find(|i| i.path.ends_with("old.bin")).unwrap();
        assert_eq!(file.modified, 1_000_000_000);
        let dir = r.top_dirs.iter().find(|i| i.path.ends_with("proj")).unwrap();
        assert_eq!(dir.modified, 1_600_000_000);
    }

    #[test]
    fn hidden_files_included() {
        let s = Sandbox::new("hidden");
        let f = s.file("secret.bin", 1234);
        assert!(Command::new("attrib").arg("+h").arg(&f).status().unwrap().success());
        assert_eq!(run(&s.0).bytes, 1234);
    }

    #[test]
    fn junctions_not_followed() {
        let s = Sandbox::new("junction");
        s.file("real/big.bin", 5000);
        let ok = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(s.0.join("link"))
            .arg(s.0.join("real"))
            .output()
            .unwrap();
        assert!(ok.status.success());
        assert_eq!(run(&s.0).bytes, 5000, "la jonction ne doit pas doubler la taille");
    }

    #[test]
    fn pass_through_dirs_skipped() {
        let s = Sandbox::new("passthrough");
        s.file("outer/inner/a.bin", 1000);
        s.file("outer/inner/b.bin", 1000);
        let r = run(&s.0);
        let dirs: Vec<&str> = r.top_dirs.iter().map(|i| i.path.as_str()).collect();
        assert!(dirs.iter().any(|d| d.ends_with("inner")));
        assert!(!dirs.iter().any(|d| d.ends_with("outer")), "outer n'est qu'un passage : {dirs:?}");
    }

    #[test]
    fn unreadable_root_counted_as_denied() {
        let r = run(Path::new(r"C:\definitely-not-here-kleaner"));
        assert_eq!((r.bytes, r.denied), (0, 1));
    }

    #[test]
    fn cancel_stops_scan() {
        let s = Sandbox::new("cancel");
        s.file("a.bin", 10);
        let r = scan(std::slice::from_ref(&s.0),&AtomicBool::new(true), &Progress::default());
        assert!(r.cancelled);
        assert_eq!(r.files, 0);
    }

    #[test]
    fn top_n_keeps_largest() {
        let t = TopN::new();
        for i in 0..(TOP_N as u64 * 3) {
            t.offer(i + 1, Path::new("x"), 0);
        }
        let v = t.into_sorted();
        assert_eq!(v.len(), TOP_N);
        assert_eq!(v[0].size, TOP_N as u64 * 3);
        assert_eq!(v[TOP_N - 1].size, TOP_N as u64 * 2 + 1);
    }

    /// Mesure sur la vraie machine : `cargo test --release scan::tests::bench -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn bench_fixed_drives() {
        let r = scan(&fixed_drives(), &AtomicBool::new(false), &Progress::default());
        println!(
            "{:?} : {} fichiers, {:.1} Go, {} dossiers refusés, {} ms",
            r.roots,
            r.files,
            r.bytes as f64 / (1u64 << 30) as f64,
            r.denied,
            r.duration_ms
        );
        for i in r.top_dirs.iter().take(10) {
            println!("  dossier {:>8.2} Go  {}", i.size as f64 / (1u64 << 30) as f64, i.path);
        }
        for i in r.top_files.iter().take(5) {
            println!("  fichier {:>8.2} Go  {}", i.size as f64 / (1u64 << 30) as f64, i.path);
        }
    }

    #[test]
    fn c_drive_is_fixed() {
        assert!(fixed_drives().contains(&PathBuf::from("C:\\")));
    }
}
