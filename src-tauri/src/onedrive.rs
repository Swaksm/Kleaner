//! Cloud (OneDrive et Dropbox) : gros fichiers synchronisés mais aussi gardés sur ce PC.
//! « Libérer de l'espace » = l'action officielle de Windows : attributs Unpinned (+U) et non Pinned (-P),
//! le fournisseur retire alors la copie locale ; le fichier reste en ligne et se retélécharge à l'ouverture.

use crate::journal;
use rayon::prelude::*;
use serde::Serialize;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

const PINNED: u32 = 0x0008_0000;
const UNPINNED: u32 = 0x0010_0000;
const RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
const OFFLINE: u32 = 0x1000;
const REPARSE: u32 = 0x400;
pub const MIN_SIZE: u64 = 20 * 1024 * 1024;

#[derive(Serialize, Clone, Debug)]
pub struct CloudFile {
    pub path: String,
    pub size: u64,
    pub modified: u64,
    /// « Toujours garder sur cet appareil » : l'utilisateur l'a voulu ainsi.
    pub pinned: bool,
    /// OneDrive ou Dropbox.
    pub provider: &'static str,
}

/// Dossiers OneDrive (personnel et professionnel) définis par Windows.
fn onedrive_roots() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"]
        .iter()
        .filter_map(|k| std::env::var_os(k).map(PathBuf::from))
        .filter(|p| p.is_dir())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Dossiers Dropbox déclarés dans `info.json`. Un compte personnel gratuit (Basic) n'a pas la synchronisation
/// intelligente : « libérer de l'espace » n'y ferait rien, on ne le propose pas.
fn dropbox_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for var in ["LOCALAPPDATA", "APPDATA"] {
        let Some(base) = std::env::var_os(var) else { continue };
        let Ok(text) = std::fs::read_to_string(PathBuf::from(base).join("Dropbox").join("info.json")) else { continue };
        let Ok(info) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        for kind in ["personal", "business"] {
            let Some(acc) = info.get(kind) else { continue };
            let basic = acc.get("subscription_type").and_then(|s| s.as_str()).is_some_and(|s| s.eq_ignore_ascii_case("basic"));
            if let Some(p) = acc.get("path").and_then(|p| p.as_str()).map(PathBuf::from).filter(|p| p.is_dir() && !basic) {
                v.push(p);
            }
        }
    }
    v.sort();
    v.dedup();
    v
}

/// Tous les dossiers cloud, avec leur fournisseur.
pub fn roots() -> Vec<PathBuf> {
    let mut v = onedrive_roots();
    v.extend(dropbox_roots());
    v
}

fn walk(dir: &Path, provider: &'static str, out: &mut Vec<CloudFile>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut subdirs = Vec::new();
    for e in rd.flatten() {
        let Ok(m) = e.metadata() else { continue };
        let a = m.file_attributes();
        if m.is_dir() {
            if a & REPARSE == 0 || a & (RECALL_ON_DATA_ACCESS | PINNED | UNPINNED) != 0 {
                subdirs.push(e.path());
            }
            continue;
        }
        // Déjà « en ligne uniquement » : rien à libérer.
        if a & (RECALL_ON_DATA_ACCESS | OFFLINE) != 0 || m.len() < MIN_SIZE {
            continue;
        }
        let modified = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
        out.push(CloudFile { path: e.path().display().to_string(), size: m.len(), modified, pinned: a & PINNED != 0, provider });
    }
    let found: Vec<Vec<CloudFile>> = subdirs
        .par_iter()
        .map(|d| {
            let mut v = Vec::new();
            walk(d, provider, &mut v);
            v
        })
        .collect();
    out.extend(found.into_iter().flatten());
}

/// Gros fichiers OneDrive présents sur le disque, du plus gros au plus petit.
pub fn local_files() -> Vec<CloudFile> {
    let mut out = Vec::new();
    for (list, provider) in [(onedrive_roots(), "OneDrive"), (dropbox_roots(), "Dropbox")] {
        for r in list {
            walk(&r, provider, &mut out);
        }
    }
    out.sort_by_key(|f| std::cmp::Reverse(f.size));
    out
}

fn set_attributes(path: &Path, attrs: u32) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::SetFileAttributesW;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    unsafe { SetFileAttributesW(wide.as_ptr(), attrs) != 0 }
}

/// Passe les fichiers en « en ligne uniquement ». Seuls des fichiers sous un dossier OneDrive sont acceptés.
pub fn free_up(paths: &[PathBuf], journal_dir: &Path) -> Result<(u64, u64), String> {
    let roots: Vec<String> = roots().iter().map(|r| format!("{}\\", r.display().to_string().to_lowercase())).collect();
    let (mut files, mut bytes, mut failed) = (0u64, 0u64, 0u64);
    for p in paths {
        let lower = p.display().to_string().to_lowercase();
        if !roots.iter().any(|r| lower.starts_with(r)) {
            return Err(format!("Refused: {} is not in a OneDrive or Dropbox folder", p.display()));
        }
        let m = std::fs::symlink_metadata(p).map_err(|e| format!("{}: {e}", p.display()))?;
        // Garder les autres attributs ; retirer « toujours garder », ajouter « libérer ».
        let attrs = (m.file_attributes() & !PINNED) | UNPINNED;
        if set_attributes(p, attrs) {
            files += 1;
            bytes += m.len();
        } else {
            failed += 1;
        }
    }
    let mut j = journal::Entry::new("cloud-free-up");
    j.paths = paths.iter().map(|p| p.display().to_string()).collect();
    j.files = files;
    j.bytes = bytes;
    j.skipped = failed;
    journal::append(journal_dir, &j).map_err(|e| format!("Log: {e}"))?;
    Ok((files, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outside_onedrive_refused() {
        let d = std::env::temp_dir().join(format!("kleaner-od-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("x.bin");
        std::fs::write(&f, b"x").unwrap();
        let err = free_up(&[f], &d).unwrap_err();
        assert!(err.starts_with("Refused"), "{err}");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
