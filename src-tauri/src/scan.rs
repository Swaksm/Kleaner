//! Analyse des disques : parcours parallèle, fichiers cachés compris, liens et jonctions ignorés.
//! Lecture seule : rien n'est modifié.

use crate::mft;
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
/// En dessous : miniatures, icônes, extraits sans intérêt pour l'utilisateur.
const VIDEO_MIN: u64 = 1024 * 1024;
const IMAGE_MIN: u64 = 50 * 1024;
/// Dossiers où les médias appartiennent aux logiciels, pas à l'utilisateur.
const MEDIA_SKIP_VARS: [&str; 6] = ["SystemRoot", "ProgramFiles", "ProgramFiles(x86)", "ProgramData", "LOCALAPPDATA", "APPDATA"];
const MEDIA_SKIP_NAMES: [&str; 2] = ["$recycle.bin", "system volume information"];
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
/// Fichiers « en ligne uniquement » (OneDrive…) : ils n'occupent pas le disque.
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
/// Doublons : seuls les fichiers de l'utilisateur d'au moins 1 Mo sont comparés.
const DUP_MIN: u64 = 1024 * 1024;
/// Carte du disque : dossiers d'au moins 64 Mo.
const TREE_MIN: u64 = 64 * 1024 * 1024;
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

pub fn category(path: &Path) -> Category {
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
    /// Analyse faite en admin avec le privilège de sauvegarde (dossiers protégés inclus).
    pub full_access: bool,
    /// Disques lus par la table des fichiers NTFS (MFT), bien plus vite : admin seulement.
    pub fast: bool,
    pub duration_ms: u64,
    pub top_files: Vec<Item>,
    pub top_dirs: Vec<Item>,
    /// Toutes les vidéos et photos personnelles (hors dossiers système et logiciels), par taille.
    pub videos: Vec<Item>,
    pub images: Vec<Item>,
    pub categories: Vec<CategoryTotal>,
    /// Taille et espace libre de chaque disque analysé.
    pub drives: Vec<DriveSpace>,
    /// Dossiers régénérables des projets (node_modules, target, .venv…).
    pub artifacts: Vec<Artifact>,
    /// Dossiers d'au moins 64 Mo, pour la carte du disque.
    pub tree: Vec<Item>,
}

/// Dossier de build ou de dépendances d'un projet, recréé par l'outil du projet.
#[derive(Serialize, Clone, Debug)]
pub struct Artifact {
    pub path: String,
    /// node, python, rust, gradle, build, dart, next, dotnet
    pub kind: &'static str,
    pub size: u64,
    pub project: String,
    /// Dernière modification des fichiers du projet, hors dossier régénérable.
    pub project_modified: u64,
}

/// Fichier de l'utilisateur candidat à la recherche de doublons.
#[derive(Clone, Debug)]
pub struct DupCandidate {
    pub path: PathBuf,
    pub size: u64,
    pub modified: u64,
}

/// Contenu personnel (photos, vidéos, musique, documents, archives, installeurs) : seul candidat aux doublons.
/// Les .dll, .exe, bases de données et fichiers de jeux en double sont normaux : les supprimer casse le logiciel.
fn user_content(path: &Path, cat: Category) -> bool {
    const DOCS: [&str; 12] = ["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "epub", "rtf"];
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    !matches!(cat, Category::Other) || DOCS.contains(&ext.as_str())
}

/// Type de dossier régénérable, d'après son nom et les fichiers du projet à côté.
fn artifact_kind(name: &str, files: &[String]) -> Option<&'static str> {
    let has = |f: &str| files.iter().any(|x| x == f);
    let has_ext = |e: &str| files.iter().any(|x| x.ends_with(e));
    match name {
        "node_modules" if has("package.json") => Some("node"),
        ".next" if has("package.json") => Some("next"),
        ".venv" => Some("python"),
        "venv" if has("requirements.txt") || has("pyproject.toml") || has("setup.py") || has("pipfile") => Some("python"),
        "target" if has("cargo.toml") => Some("rust"),
        ".gradle" if has("build.gradle") || has("build.gradle.kts") || has("settings.gradle") || has("settings.gradle.kts") => {
            Some("gradle")
        }
        "build" if has("build.gradle") || has("build.gradle.kts") || has("pubspec.yaml") || has("cmakelists.txt") => Some("build"),
        ".dart_tool" if has("pubspec.yaml") => Some("dart"),
        "obj" | "bin" if has_ext(".csproj") || has_ext(".fsproj") || has_ext(".vbproj") => Some("dotnet"),
        _ => None,
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct DriveSpace {
    pub root: String,
    pub total: u64,
    pub free: u64,
}

/// Taille et espace libre d'un disque (GetDiskFreeSpaceExW). `None` si illisible.
pub fn drive_space(root: &Path) -> Option<DriveSpace> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = root.as_os_str().encode_wide().chain([0]).collect();
    let (mut free, mut total) = (0u64, 0u64);
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, std::ptr::null_mut()) };
    (ok != 0).then(|| DriveSpace { root: root.display().to_string(), total, free })
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
    /// Privilège de sauvegarde actif : les dossiers refusés sont relus en mode sauvegarde.
    backup: bool,
    progress: &'a Progress,
    files: TopN,
    dirs: TopN,
    cats: [(AtomicU64, AtomicU64); 6],
    media_skip: Vec<String>,
    videos: Mutex<Vec<Item>>,
    images: Mutex<Vec<Item>>,
    dups: Mutex<Vec<DupCandidate>>,
    artifacts: Mutex<Vec<Artifact>>,
    tree: Mutex<Vec<Item>>,
    profile: String,
}

impl Ctx<'_> {
    /// Les médias de ce dossier (et de ses sous-dossiers) appartiennent-ils à l'utilisateur ?
    fn media_dir(&self, dir: &Path) -> bool {
        let lower = dir.to_string_lossy().to_lowercase();
        let name = dir.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        // Dossiers cachés du profil (.vscode, .cargo…) et gestionnaires d'outils : logiciels, pas fichiers perso.
        let tool_dir = (name.starts_with('.') || name == "scoop")
            && dir.parent().is_some_and(|p| p.to_string_lossy().to_lowercase() == self.profile);
        !self.media_skip.contains(&lower) && !MEDIA_SKIP_NAMES.contains(&name.as_str()) && !tool_dir
    }
}

/// Une entrée de dossier, quelle que soit la façon dont elle a été lue.
struct Entry {
    path: PathBuf,
    is_dir: bool,
    attrs: u32,
    len: u64,
    modified: u64,
    /// Numéro d'enregistrement MFT (scan rapide) ; 0 sinon.
    frn: u32,
}

fn list_std(dir: &Path) -> Option<Vec<Entry>> {
    let rd = fs::read_dir(dir).ok()?;
    Some(
        rd.flatten()
            .filter_map(|e| {
                // Sous Windows, ces métadonnées viennent de la lecture du dossier : pas d'appel système en plus.
                let m = e.metadata().ok()?;
                Some(Entry {
                    path: e.path(),
                    is_dir: m.is_dir(),
                    attrs: m.file_attributes(),
                    len: m.len(),
                    modified: unix_secs(m.modified()),
                    frn: 0,
                })
            })
            .collect(),
    )
}

/// Liste un dossier avec FILE_FLAG_BACKUP_SEMANTICS : avec le privilège de sauvegarde actif, Windows
/// accorde la lecture même aux dossiers réservés à SYSTEM. Ouverture en lecture seule (FILE_LIST_DIRECTORY).
fn list_backup(dir: &Path) -> Option<Vec<Entry>> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FULL_DIR_INFO, FILE_LIST_DIRECTORY, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, FileFullDirectoryInfo, GetFileInformationByHandleEx, OPEN_EXISTING,
    };
    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
    // Écart entre 1601 (FILETIME) et 1970 (Unix), en unités de 100 ns.
    const EPOCH_DIFF: i64 = 116_444_736_000_000_000;

    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
    let h = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_LIST_DIRECTORY,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return None;
    }
    let mut out = Vec::new();
    let mut buf = vec![0u64; 8192]; // 64 Ko, aligné sur 8 octets
    loop {
        let ok = unsafe {
            GetFileInformationByHandleEx(h, FileFullDirectoryInfo, buf.as_mut_ptr().cast(), (buf.len() * 8) as u32)
        };
        if ok == 0 {
            break; // ERROR_NO_MORE_FILES ou erreur : on garde ce qui a été lu
        }
        let base = buf.as_ptr() as *const u8;
        let mut offset = 0usize;
        loop {
            let info = unsafe { &*(base.add(offset) as *const FILE_FULL_DIR_INFO) };
            let name_len = info.FileNameLength as usize / 2;
            let name = unsafe { std::slice::from_raw_parts(info.FileName.as_ptr(), name_len) };
            let name = String::from_utf16_lossy(name);
            if name != "." && name != ".." {
                out.push(Entry {
                    path: dir.join(&name),
                    is_dir: info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0,
                    attrs: info.FileAttributes,
                    len: info.EndOfFile.max(0) as u64,
                    modified: ((info.LastWriteTime - EPOCH_DIFF).max(0) / 10_000_000) as u64,
                    frn: 0,
                });
            }
            if info.NextEntryOffset == 0 {
                break;
            }
            offset += info.NextEntryOffset as usize;
        }
    }
    unsafe { CloseHandle(h) };
    Some(out)
}

/// Le contenu d'un dossier, lu dans l'index de la MFT (scan rapide) au lieu du disque.
fn list_mft(ix: &mft::Index, dir: &Path, frn: u32) -> Vec<Entry> {
    ix.children(frn)
        .into_iter()
        .map(|c| Entry { path: dir.join(&c.name), is_dir: c.is_dir, attrs: c.attrs, len: c.len, modified: c.modified, frn: c.frn })
        .collect()
}

fn unix_secs(t: std::io::Result<std::time::SystemTime>) -> u64 {
    t.ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs())
}

/// (taille totale, modification la plus récente) de `dir`, en enregistrant fichiers et dossiers au passage.
// shortcut: les liens physiques (WinSxS ↔ System32) sont comptés deux fois ; à corriger si les tailles de C:\Windows gênent.
/// `media` : zone de l'utilisateur (ni Windows, ni Program Files, ni AppData…).
/// `in_artifact` : à l'intérieur d'un dossier régénérable (node_modules…) ou d'un dépôt .git.
fn walk(dir: &Path, frn: u32, ix: Option<&mft::Index>, ctx: &Ctx, media: bool, in_artifact: bool) -> (u64, u64) {
    if ctx.cancel.load(Relaxed) {
        return (0, 0);
    }
    let entries = match ix {
        Some(ix) => Some(list_mft(ix, dir, frn)),
        None => list_std(dir).or_else(|| if ctx.backup { list_backup(dir) } else { None }),
    };
    let Some(entries) = entries else {
        ctx.progress.denied.fetch_add(1, Relaxed);
        return (0, 0);
    };
    let mut own = 0u64;
    let mut newest = 0u64;
    let mut subdirs = Vec::new();
    let mut sub_frns = Vec::new();
    let mut file_names = Vec::new();
    for e in entries {
        if e.attrs & FILE_ATTRIBUTE_REPARSE_POINT != 0 && e.is_dir {
            continue; // jonction ou lien : déjà compté ailleurs, ou hors disque
        }
        if e.is_dir {
            subdirs.push(e.path);
            sub_frns.push(e.frn);
            continue;
        }
        if e.attrs & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS != 0 {
            continue;
        }
        let (len, modified, path) = (e.len, e.modified, e.path);
        let cat = category(&path);
        if media {
            file_names.push(path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
            if len >= DUP_MIN && !in_artifact && user_content(&path, cat) {
                ctx.dups.lock().unwrap().push(DupCandidate { path: path.clone(), size: len, modified });
            }
        }
        own += len;
        newest = newest.max(modified);
        if media {
            let list = match cat {
                Category::Videos if len >= VIDEO_MIN => Some(&ctx.videos),
                Category::Images if len >= IMAGE_MIN => Some(&ctx.images),
                _ => None,
            };
            if let Some(list) = list {
                list.lock().unwrap().push(Item { size: len, path: path.display().to_string(), modified });
            }
        }
        let c = cat as usize;
        ctx.cats[c].0.fetch_add(len, Relaxed);
        ctx.cats[c].1.fetch_add(1, Relaxed);
        ctx.progress.files.fetch_add(1, Relaxed);
        ctx.progress.bytes.fetch_add(len, Relaxed);
        ctx.files.offer(len, &path, modified);
    }
    // Electron : « resources\app\node_modules » fait partie d'une appli installée, pas d'un projet.
    let electron_app = dir.file_name().is_some_and(|n| n.eq_ignore_ascii_case("app"))
        && dir.parent().and_then(|p| p.file_name()).is_some_and(|n| n.eq_ignore_ascii_case("resources"));
    let kinds: Vec<Option<&'static str>> = subdirs
        .iter()
        .map(|d| {
            let name = d.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            if !media || in_artifact || electron_app { None } else { artifact_kind(&name, &file_names) }
        })
        .collect();
    let children: Vec<(u64, u64)> = subdirs
        .par_iter()
        .zip(&sub_frns)
        .zip(&kinds)
        .map(|((d, &sub_frn), kind)| {
            let git = d.file_name().is_some_and(|n| n == ".git");
            walk(d, sub_frn, ix, ctx, media && ctx.media_dir(d), in_artifact || kind.is_some() || git)
        })
        .collect();
    let total = own + children.iter().map(|c| c.0).sum::<u64>();
    // Date du projet : ses fichiers, sans les dossiers régénérables (un npm install ne le « rajeunit » pas).
    let project_modified =
        children.iter().zip(&kinds).filter(|(_, k)| k.is_none()).map(|(c, _)| c.1).fold(newest, u64::max);
    for ((d, kind), c) in subdirs.iter().zip(&kinds).zip(&children) {
        if let Some(kind) = kind
            && c.0 > 0
        {
            ctx.artifacts.lock().unwrap().push(Artifact {
                path: d.display().to_string(),
                kind,
                size: c.0,
                project: dir.display().to_string(),
                project_modified,
            });
        }
    }
    if total >= TREE_MIN {
        ctx.tree.lock().unwrap().push(Item { size: total, path: dir.display().to_string(), modified: 0 });
    }
    let newest = children.iter().map(|c| c.1).fold(newest, u64::max);
    let biggest_child = children.iter().map(|c| c.0).max().unwrap_or(0);
    if total > 0 && (biggest_child as f64) < PASS_THROUGH * total as f64 {
        ctx.dirs.offer(total, dir, newest);
    }
    (total, newest)
}

pub fn scan(roots: &[PathBuf], cancel: &AtomicBool, progress: &Progress) -> ScanResult {
    scan_full(roots, cancel, progress).0
}

/// Analyse + candidats doublons (gardés côté Rust : trop nombreux pour l'interface).
pub fn scan_full(roots: &[PathBuf], cancel: &AtomicBool, progress: &Progress) -> (ScanResult, Vec<DupCandidate>) {
    let start = Instant::now();
    // Mode admin : on active le privilège de sauvegarde pour lire aussi les dossiers système protégés.
    let backup = crate::elevation::is_elevated() && crate::elevation::enable_backup_privilege();
    let ctx = Ctx {
        cancel,
        backup,
        progress,
        files: TopN::new(),
        dirs: TopN::new(),
        cats: Default::default(),
        media_skip: MEDIA_SKIP_VARS
            .iter()
            .filter_map(|k| std::env::var(k).ok())
            .map(|v| v.trim_end_matches('\\').to_lowercase())
            .collect(),
        videos: Mutex::default(),
        images: Mutex::default(),
        dups: Mutex::default(),
        artifacts: Mutex::default(),
        tree: Mutex::default(),
        profile: std::env::var("USERPROFILE").unwrap_or_default().to_lowercase(),
    };
    // Scan rapide : en admin, la table des fichiers NTFS (MFT) est lue d'un bloc ; sinon, ou au moindre doute, parcours classique.
    let indexes: Vec<Option<mft::Index>> = roots.iter().map(|r| fast_index(r, backup, cancel)).collect();
    let fast = !indexes.is_empty() && indexes.iter().all(Option::is_some);
    let bytes: u64 = roots.par_iter().zip(&indexes).map(|(r, ix)| walk(r, mft::ROOT, ix.as_ref(), &ctx, true, false).0).sum();
    // Une racine de disque dans le top n'apprend rien à l'utilisateur.
    let roots_str: Vec<String> = roots.iter().map(|r| r.display().to_string()).collect();
    let mut top_dirs = ctx.dirs.into_sorted();
    top_dirs.retain(|i| !roots_str.contains(&i.path));
    let categories = CATEGORIES
        .iter()
        .zip(&ctx.cats)
        .map(|(&category, (b, f))| CategoryTotal { category, bytes: b.load(Relaxed), files: f.load(Relaxed) })
        .collect();
    let mut artifacts = ctx.artifacts.into_inner().unwrap();
    artifacts.sort_by_key(|a| Reverse(a.size));
    let mut tree = ctx.tree.into_inner().unwrap();
    tree.sort_by_key(|i| Reverse(i.size));
    let result = ScanResult {
        roots: roots_str,
        bytes,
        files: progress.files.load(Relaxed),
        denied: progress.denied.load(Relaxed),
        cancelled: cancel.load(Relaxed),
        full_access: backup,
        fast,
        duration_ms: start.elapsed().as_millis() as u64,
        top_files: ctx.files.into_sorted(),
        top_dirs,
        videos: sorted(ctx.videos),
        images: sorted(ctx.images),
        categories,
        drives: roots.iter().filter_map(|r| drive_space(r)).collect(),
        artifacts,
        tree,
    };
    (result, ctx.dups.into_inner().unwrap())
}

/// Index MFT d'une racine de disque (`C:\`), si le scan rapide est possible.
fn fast_index(root: &Path, admin: bool, cancel: &AtomicBool) -> Option<mft::Index> {
    if !admin || root.parent().is_some() {
        return None;
    }
    let used = drive_space(root).map_or(0, |d| d.total.saturating_sub(d.free));
    mft::build(root, cancel, used)
}

fn sorted(list: Mutex<Vec<Item>>) -> Vec<Item> {
    let mut v = list.into_inner().unwrap();
    v.sort_by(|a, b| b.cmp(a));
    v
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DriveKind {
    Fixed,
    Removable,
    Network,
    Cdrom,
    Other,
}

#[derive(Serialize, Clone, Debug)]
pub struct DriveInfo {
    pub root: String,
    pub kind: DriveKind,
    pub total: u64,
    pub free: u64,
}

/// Tous les disques présents, avec leur type et leur espace (lecteurs vides ignorés).
pub fn drives() -> Vec<DriveInfo> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| format!("{}:\\", (b'A' + i) as char))
        .filter_map(|root| {
            let wide: Vec<u16> = root.encode_utf16().chain([0]).collect();
            let kind = match unsafe { GetDriveTypeW(wide.as_ptr()) } {
                2 => DriveKind::Removable,
                3 => DriveKind::Fixed,
                4 => DriveKind::Network,
                5 => DriveKind::Cdrom,
                _ => DriveKind::Other,
            };
            // Lecteur de CD vide ou carte absente : pas d'espace lisible.
            let space = drive_space(Path::new(&root))?;
            Some(DriveInfo { root, kind, total: space.total, free: space.free })
        })
        .collect()
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
    fn backup_listing_matches_standard_listing() {
        let s = Sandbox::new("backuplist");
        s.file("a.bin", 1234);
        s.file("sub/b.bin", 10);
        let key = |v: Vec<Entry>| {
            let mut v: Vec<(String, bool, u64, u64)> = v
                .into_iter()
                .map(|e| (e.path.display().to_string(), e.is_dir, if e.is_dir { 0 } else { e.len }, e.modified))
                .collect();
            v.sort();
            v
        };
        assert_eq!(key(list_backup(&s.0).unwrap()), key(list_std(&s.0).unwrap()));
    }

    #[test]
    fn media_collected_with_size_floor() {
        let s = Sandbox::new("media");
        s.file("Vacances/film.mp4", 2 * 1024 * 1024);
        s.file("Vacances/extrait.mp4", 1000);
        s.file("Vacances/photo.JPG", 60 * 1024);
        s.file("Vacances/icone.png", 2000);
        let r = run(&s.0);
        assert_eq!(r.videos.len(), 1);
        assert!(r.videos[0].path.ends_with("film.mp4"));
        assert_eq!(r.images.len(), 1);
        assert!(r.images[0].path.ends_with("photo.JPG"));
    }

    #[test]
    fn media_skipped_in_software_folders() {
        let s = Sandbox::new("mediaskip");
        s.file("$Recycle.Bin/x/film.mp4", 2 * 1024 * 1024);
        assert!(run(&s.0).videos.is_empty());
    }

    #[test]
    fn project_artifacts_detected_with_project_date() {
        let s = Sandbox::new("artifacts");
        let pkg = s.file("web/package.json", 10);
        s.file("web/node_modules/react/index.js", 5000);
        s.file("web/node_modules/react/node_modules/x/y.js", 100);
        s.file("rs/Cargo.toml", 10);
        s.file("rs/target/debug/app.exe", 3000);
        s.file("notaproject/target/keep.bin", 1000);
        let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_500_000_000);
        fs::File::options().write(true).open(&pkg).unwrap().set_modified(old).unwrap();
        let r = run(&s.0);
        let find = |k: &str| r.artifacts.iter().filter(|a| a.kind == k).collect::<Vec<_>>();
        let node = find("node");
        assert_eq!(node.len(), 1, "node_modules imbriqué non compté : {:?}", r.artifacts);
        assert_eq!(node[0].size, 5100);
        assert_eq!(node[0].project_modified, 1_500_000_000, "la date du projet ignore node_modules");
        assert_eq!(find("rust").len(), 1);
        assert!(!r.artifacts.iter().any(|a| a.path.contains("notaproject")), "target sans Cargo.toml ignoré");
    }

    #[test]
    fn electron_app_node_modules_ignored() {
        let s = Sandbox::new("electron");
        s.file("MyApp/resources/app/package.json", 10);
        s.file("MyApp/resources/app/node_modules/x/i.js", 100);
        assert!(run(&s.0).artifacts.is_empty());
    }

    #[test]
    fn dup_candidates_collected() {
        let s = Sandbox::new("dups");
        s.file("a/big.mp4", 2 * 1024 * 1024);
        s.file("a/small.mp4", 1000);
        s.file("a/lib.dll", 2 * 1024 * 1024);
        s.file("p/package.json", 1);
        s.file("p/node_modules/m/big.mp4", 2 * 1024 * 1024);
        let (_, d) = scan_full(std::slice::from_ref(&s.0), &AtomicBool::new(false), &Progress::default());
        assert_eq!(d.len(), 1, "{d:?}");
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
    fn c_drive_space_is_consistent() {
        let d = drive_space(Path::new("C:\\")).unwrap();
        assert!(d.total > 0 && d.free <= d.total);
    }

    #[test]
    fn drives_lists_c_as_fixed() {
        let c = drives().into_iter().find(|d| d.root == "C:\\").unwrap();
        assert_eq!(c.kind, DriveKind::Fixed);
    }

    #[test]
    fn c_drive_is_fixed() {
        assert!(fixed_drives().contains(&PathBuf::from("C:\\")));
    }
}
