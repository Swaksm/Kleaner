pub mod guard;
pub mod journal;

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::Path;
use tauri::Manager;

#[derive(Serialize)]
struct JournalView {
    path: String,
    entries: Vec<journal::Entry>,
}

#[tauri::command]
fn read_journal(limit: usize) -> Result<JournalView, String> {
    let dir = journal::data_dir();
    let entries = journal::read_last(&dir, limit).map_err(|e| e.to_string())?;
    Ok(JournalView { path: dir.join("journal.log").display().to_string(), entries })
}

/// Efface toutes les données de Kleaner puis quitte. L'exe portable reste à supprimer à la main.
#[tauri::command]
fn remove_kleaner_data(app: tauri::AppHandle) -> Result<(), String> {
    let ours = journal::data_dir();
    // Dossier WebView2 (localStorage, cache) : %LOCALAPPDATA%\<identifier>.
    let webview = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let named = |p: &Path, name: &str| p.is_absolute() && p.file_name().is_some_and(|n| n == name);
    if !named(&ours, "Kleaner") || !named(&webview, &app.config().identifier) {
        return Err("Dossiers de données inattendus, rien n'a été supprimé.".into());
    }
    // WebView2 verrouille ses fichiers tant que l'appli tourne : on supprime 2 s après la fermeture.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("cmd")
        .raw_arg(format!(
            "/C ping -n 3 127.0.0.1 >nul & rmdir /S /Q \"{}\" & rmdir /S /Q \"{}\"",
            ours.display(),
            webview.display()
        ))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![read_journal, remove_kleaner_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
