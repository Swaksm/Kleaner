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

/// Envoie des dossiers entiers à la Corbeille (restes de programmes désinstallés).
/// Tout ou rien : si un seul dossier est refusé par la garde, rien n'est déplacé.
pub fn trash_dirs(paths: &[PathBuf], protected: &guard::Protected, journal_dir: &Path, sizes: &[u64]) -> Result<ActionReport, String> {
    let mut checked = Vec::with_capacity(paths.len());
    for p in paths {
        let c = guard::check_trash(p, protected).map_err(|e| format!("Refused: {} ({e})", p.display()))?;
        if !std::fs::metadata(&c).is_ok_and(|m| m.is_dir()) {
            return Err(format!("Refused: {} (not a folder)", p.display()));
        }
        checked.push(c);
    }
    let bytes = sizes.iter().sum();
    let mut entry = journal::Entry::new("trash-leftovers");
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

/// Windows peut masquer l'extension dans le nom renvoyé (« photo » pour « photo.jpg ») :
/// on compare le dossier, puis le nom complet, ou le nom sans extension + l'extension de l'identifiant interne.
fn same_item(i: &trash::TrashItem, path: &Path) -> bool {
    let eq = |a: &std::ffi::OsStr, b: &str| a.to_string_lossy().eq_ignore_ascii_case(b);
    let Some(parent) = path.parent() else { return false };
    // Forme longue des deux côtés : « RUNNER~1 » et « runneradmin » désignent le même dossier.
    let full = |p: &Path| {
        std::fs::canonicalize(p)
            .map(|c| PathBuf::from(c.to_string_lossy().trim_start_matches(r"\\?\").to_string()))
            .unwrap_or_else(|_| p.to_path_buf())
    };
    if !full(&i.original_parent).as_os_str().eq_ignore_ascii_case(full(parent).as_os_str()) {
        return false;
    }
    let file = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let stem = path.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy().to_lowercase()));
    eq(&i.name, &file) || (eq(&i.name, &stem) && ext.is_some_and(|e| i.id.to_string_lossy().to_lowercase().ends_with(&e)))
}

/// Remet à leur place des éléments envoyés à la Corbeille par Kleaner (Journal → Restaurer).
/// Pour chaque chemin, l'élément le plus récemment supprimé à cet endroit est restauré.
pub fn restore_from_trash(paths: &[String], journal_dir: &Path) -> Result<ActionReport, String> {
    let items = trash::os_limited::list().map_err(|e| e.to_string())?;
    let mut chosen = Vec::new();
    let mut missing = 0;
    for p in paths {
        let path = Path::new(p);
        let found = items.iter().filter(|i| same_item(i, path)).max_by_key(|i| i.time_deleted);
        match found {
            // Le crate restaure avec le nom renvoyé par Windows, parfois sans extension : on remet le vrai nom.
            Some(i) => chosen.push(trash::TrashItem { name: path.file_name().unwrap_or(&i.name).to_os_string(), ..i.clone() }),
            None => missing += 1,
        }
    }
    let mut entry = journal::Entry::new("restore");
    entry.paths = chosen.iter().map(|i| i.original_path().display().to_string()).collect();
    entry.files = chosen.len() as u64;
    entry.skipped = missing;
    let result = if chosen.is_empty() { Ok(()) } else { trash::os_limited::restore_all(chosen).map_err(|e| e.to_string()) };
    if let Err(e) = &result {
        entry.error = Some(e.clone());
    }
    journal::append(journal_dir, &entry).map_err(|e| format!("Log: {e}"))?;
    result.map(|_| ActionReport { files: entry.files, bytes: 0 })
}

/// Rapport de bug : version, Windows, droits, dernières actions et erreurs du journal.
/// Enregistré dans les rapports de Kleaner ; rien n'est envoyé.
pub fn bug_report(version: &str, data_dir: &Path) -> Result<PathBuf, String> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    use std::os::windows::process::CommandExt;
    let windows = std::process::Command::new("cmd")
        .args(["/C", "ver"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let entries = journal::read_last(data_dir, 50).unwrap_or_default();
    let errors: Vec<&journal::Entry> = entries.iter().filter(|e| e.error.is_some()).collect();
    let mut text = format!(
        "# Kleaner bug report\n\n- Kleaner: {version}\n- Windows: {windows}\n- Admin: {}\n- Architecture: {}\n\n## Errors ({})\n\n",
        crate::elevation::is_elevated(),
        std::env::consts::ARCH,
        errors.len()
    );
    for e in &errors {
        text.push_str(&format!("- {} {}: {}\n", e.ts, e.action, e.error.as_deref().unwrap_or("")));
    }
    text.push_str("\n## Last actions (newest first)\n\n");
    for e in &entries {
        text.push_str(&format!("- {} {} files={} bytes={} skipped={}{}\n", e.ts, e.action, e.files, e.bytes, e.skipped, if e.error.is_some() { " ERROR" } else { "" }));
    }
    // Les chemins du journal ne sont pas inclus : ils peuvent contenir des noms personnels.
    text.push_str("\n(File paths are not included. Describe what you did and what happened in the GitHub issue.)\n");
    let dir = data_dir.join("rapports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("kleaner-bug-{}.md", journal::Entry::new("x").ts));
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
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
    fn trash_then_restore_brings_file_back() {
        let d = temp("restore");
        let f = d.join("restore-me.txt");
        fs::write(&f, b"hello").unwrap();
        trash_files(std::slice::from_ref(&f), &guard::Protected::from_env(), &d).unwrap();
        assert!(!f.exists());
        let r = restore_from_trash(&[f.display().to_string()], &d).unwrap();
        assert_eq!(r.files, 1);
        assert_eq!(fs::read(&f).unwrap(), b"hello");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn bug_report_has_version_and_no_paths() {
        let d = temp("bug");
        let mut e = journal::Entry::new("trash");
        e.paths = vec![r"C:\Users\secret\x".into()];
        e.error = Some("boom".into());
        journal::append(&d, &e).unwrap();
        let p = bug_report("9.9.9", &d).unwrap();
        let text = fs::read_to_string(p).unwrap();
        assert!(text.contains("9.9.9") && text.contains("boom"));
        assert!(!text.contains("secret"));
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn appdata_roots_themselves_refused() {
        let d = temp("appdataroot");
        let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
        let err = trash_dirs(&[local], &guard::Protected::from_env(), &d, &[0]).unwrap_err();
        assert!(err.starts_with("Refused"), "{err}");
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
