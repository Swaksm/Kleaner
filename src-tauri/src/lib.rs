pub mod guard;
pub mod journal;
pub mod scan;

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::Duration;
use tauri::{Emitter, Manager, State};

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

#[derive(Default)]
struct ScanState {
    cancel: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
}

#[derive(Serialize, Clone)]
struct ScanProgress {
    files: u64,
    bytes: u64,
    denied: u64,
}

/// Lance l'analyse de tous les disques fixes en arrière-plan.
/// Événements : `scan-progress` toutes les 250 ms, puis `scan-done` avec le résultat.
#[tauri::command]
fn start_scan(app: tauri::AppHandle, state: State<ScanState>) -> Result<(), String> {
    if state.running.swap(true, Relaxed) {
        return Err("Analyse déjà en cours".into());
    }
    state.cancel.store(false, Relaxed);
    let (cancel, running) = (state.cancel.clone(), state.running.clone());
    std::thread::spawn(move || {
        let progress = Arc::new(scan::Progress::default());
        let done = Arc::new(AtomicBool::new(false));
        let ticker = {
            let (app, progress, done) = (app.clone(), progress.clone(), done.clone());
            std::thread::spawn(move || {
                while !done.load(Relaxed) {
                    let p = ScanProgress {
                        files: progress.files.load(Relaxed),
                        bytes: progress.bytes.load(Relaxed),
                        denied: progress.denied.load(Relaxed),
                    };
                    let _ = app.emit("scan-progress", p);
                    std::thread::sleep(Duration::from_millis(250));
                }
            })
        };
        let result = scan::scan(&scan::fixed_drives(), &cancel, &progress);
        done.store(true, Relaxed);
        let _ = ticker.join();
        let _ = app.emit("scan-done", &result);
        running.store(false, Relaxed);
    });
    Ok(())
}

#[tauri::command]
fn cancel_scan(state: State<ScanState>) {
    state.cancel.store(true, Relaxed);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ScanState::default())
        .invoke_handler(tauri::generate_handler![read_journal, remove_kleaner_data, start_scan, cancel_scan])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
