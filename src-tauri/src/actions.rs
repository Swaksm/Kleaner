//! Actions qui modifient le disque. Chacune passe par `guard` et écrit dans le journal.

use crate::{guard, journal};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Debug, PartialEq)]
pub struct ActionReport {
    pub files: u64,
    pub bytes: u64,
}

/// Envoie des fichiers choisis par l'utilisateur à la Corbeille.
/// Tout ou rien : si un seul chemin est refusé par la garde, rien n'est déplacé.
pub fn trash_files(paths: &[PathBuf], protected: &guard::Protected, journal_dir: &Path) -> Result<ActionReport, String> {
    let mut checked = Vec::with_capacity(paths.len());
    let mut bytes = 0;
    for p in paths {
        let c = guard::check_trash(p, protected).map_err(|e| format!("Refused: {} ({e})", p.display()))?;
        let m = std::fs::metadata(&c).map_err(|e| format!("{} : {e}", p.display()))?;
        if !m.is_file() {
            return Err(format!("Refused: {} (only files can be sent to the Recycle Bin)", p.display()));
        }
        bytes += m.len();
        checked.push(c);
    }

    let mut entry = journal::Entry::new("trash");
    entry.paths = checked.iter().map(|p| p.display().to_string()).collect();
    entry.files = checked.len() as u64;
    entry.bytes = bytes;
    let result = trash::delete_all(&checked).map_err(|e| e.to_string());
    if let Err(e) = &result {
        entry.error = Some(e.clone());
    }
    journal::append(journal_dir, &entry).map_err(|e| format!("Log: {e}"))?;
    result.map(|_| ActionReport { files: entry.files, bytes })
}

/// Enregistre un rapport Markdown dans `<data_dir>\rapports` et le journalise. Renvoie son chemin.
pub fn save_report(content: &str, data_dir: &Path) -> Result<PathBuf, String> {
    let mut entry = journal::Entry::new("report");
    let dir = data_dir.join("rapports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("kleaner-rapport-{}.md", entry.ts));
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    entry.paths = vec![path.display().to_string()];
    entry.bytes = content.len() as u64;
    journal::append(data_dir, &entry).map_err(|e| format!("Log: {e}"))?;
    Ok(path)
}

/// Vérifie qu'un chemin est une photo ou une vidéo locale existante (pas un lien, pas un exe).
/// Seuls ces fichiers peuvent être affichés dans Kleaner ou ouverts avec le lecteur par défaut.
pub fn check_media(path: &Path) -> Result<PathBuf, String> {
    use crate::scan::{Category, category};
    use std::os::windows::fs::MetadataExt;
    if !matches!(category(path), Category::Videos | Category::Images) {
        return Err(format!("Not a photo or video: {}", path.display()));
    }
    // Chemin local uniquement (pas de partage réseau).
    if !path.is_absolute() || path.to_string_lossy().starts_with(r"\\") {
        return Err(format!("Not a local path: {}", path.display()));
    }
    let m = std::fs::symlink_metadata(path).map_err(|e| format!("{} : {e}", path.display()))?;
    if !m.is_file() || m.file_attributes() & 0x400 != 0 {
        return Err(format!("Not a regular file: {}", path.display()));
    }
    Ok(path.to_path_buf())
}

/// Ouvre une photo ou une vidéo avec l'application par défaut de Windows.
pub fn open_media(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let file = check_media(path)?;
    let w = |s: &std::ffi::OsStr| s.encode_wide().chain([0]).collect::<Vec<u16>>();
    let (verb, file) = (w("open".as_ref()), w(file.as_os_str()));
    let r = unsafe {
        ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL)
    };
    if r as isize > 32 { Ok(()) } else { Err("No app can open this file.".into()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kleaner-actions-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn protected_path_refused_and_nothing_logged() {
        let d = temp("protected");
        let win = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("win.ini");
        let err = trash_files(&[win], &guard::Protected::from_env(), &d).unwrap_err();
        assert!(err.starts_with("Refused"), "{err}");
        assert!(journal::read_last(&d, 10).unwrap().is_empty());
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn report_saved_under_data_dir_and_logged() {
        let d = temp("report");
        let p = save_report("# Rapport", &d).unwrap();
        assert!(p.starts_with(d.join("rapports")));
        assert_eq!(fs::read_to_string(&p).unwrap(), "# Rapport");
        assert_eq!(journal::read_last(&d, 1).unwrap()[0].action, "report");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn only_real_media_files_can_be_previewed() {
        let d = temp("media");
        let photo = d.join("photo.JPG");
        fs::write(&photo, b"x").unwrap();
        let exe = d.join("virus.exe");
        fs::write(&exe, b"x").unwrap();
        assert!(check_media(&photo).is_ok());
        assert!(check_media(&exe).unwrap_err().starts_with("Not a photo"));
        assert!(check_media(&d.join("absent.mp4")).is_err());
        assert!(check_media(Path::new(r"\\serveur\partage\film.mp4")).is_err());
        assert!(check_media(Path::new("relatif.mp4")).is_err());
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn batch_is_all_or_nothing() {
        let d = temp("batch");
        let ok = d.join("a.bin");
        fs::write(&ok, b"x").unwrap();
        let dir = d.join("folder");
        fs::create_dir(&dir).unwrap();
        let err = trash_files(&[ok.clone(), dir], &guard::Protected::from_env(), &d).unwrap_err();
        assert!(err.contains("only files"), "{err}");
        assert!(ok.exists(), "le fichier valide ne doit pas avoir bougé");
        fs::remove_dir_all(&d).unwrap();
    }
}
