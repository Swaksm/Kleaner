pub mod actions;
pub mod catalog;
pub mod duplicates;
pub mod elevation;
pub mod guard;
pub mod journal;
pub mod knowledge;
pub mod leftovers;
pub mod programs;
pub mod scan;

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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
        return Err("Unexpected data folders, nothing was deleted.".into());
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
    /// Résultats de la dernière analyse gardés côté Rust (trop gros, ou vérifiés avant suppression).
    dups: Arc<Mutex<Vec<scan::DupCandidate>>>,
    artifacts: Arc<Mutex<Vec<String>>>,
    dup_cancel: Arc<AtomicBool>,
    leftovers: Arc<Mutex<Vec<leftovers::Leftover>>>,
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
fn start_scan(app: tauri::AppHandle, state: State<ScanState>, roots: Option<Vec<String>>) -> Result<(), String> {
    // Seules des racines de disques réellement présents sont acceptées ; par défaut, les disques fixes.
    let roots: Vec<PathBuf> = match roots {
        None => scan::fixed_drives(),
        Some(asked) => {
            let present: Vec<String> = scan::drives().into_iter().map(|d| d.root).collect();
            let mut out = Vec::new();
            for r in asked {
                if !present.iter().any(|p| p.eq_ignore_ascii_case(&r)) {
                    return Err(format!("Unknown drive: {r}"));
                }
                out.push(PathBuf::from(r));
            }
            out
        }
    };
    if roots.is_empty() {
        return Err("No drive selected".into());
    }
    if state.running.swap(true, Relaxed) {
        return Err("A scan is already running".into());
    }
    state.cancel.store(false, Relaxed);
    let (cancel, running) = (state.cancel.clone(), state.running.clone());
    let (dups, artifacts) = (state.dups.clone(), state.artifacts.clone());
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
        let (result, candidates) = scan::scan_full(&roots, &cancel, &progress);
        *dups.lock().unwrap() = candidates;
        *artifacts.lock().unwrap() = result.artifacts.iter().map(|a| a.path.clone()).collect();
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

#[tauri::command]
fn move_to_trash(paths: Vec<String>) -> Result<actions::ActionReport, String> {
    let paths: Vec<_> = paths.into_iter().map(std::path::PathBuf::from).collect();
    actions::trash_files(&paths, &guard::Protected::from_env(), &journal::data_dir())
}

#[derive(Serialize)]
struct CleanersView {
    elevated: bool,
    cleaners: &'static [catalog::Cleaner],
}

#[tauri::command]
fn list_cleaners() -> CleanersView {
    CleanersView { elevated: elevation::is_elevated(), cleaners: catalog::all() }
}

/// Estimation de chaque entrée (lecture seule). Entrées admin ignorées sans droits admin.
#[tauri::command]
async fn estimate_cleaners() -> Result<Vec<catalog::CleanReport>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let elevated = elevation::is_elevated();
        catalog::all()
            .iter()
            .filter(|c| elevated || !c.admin)
            .map(|c| {
                let mut r = catalog::estimate(c);
                r.blocked_by = catalog::running(&c.close);
                r
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn run_cleaners(ids: Vec<String>) -> Result<Vec<catalog::CleanReport>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = journal::data_dir();
        ids.iter()
            .map(|id| match catalog::get(id) {
                None => catalog::CleanReport { id: id.clone(), error: Some("Unknown item".into()), ..Default::default() },
                Some(c) => catalog::execute(c, &dir).unwrap_or_else(|e| catalog::CleanReport {
                    id: id.clone(),
                    error: Some(e),
                    ..Default::default()
                }),
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

/// Relance Kleaner en admin puis ferme cette instance. Si l'UAC est refusé, on reste ouvert.
#[tauri::command]
fn is_elevated() -> bool {
    elevation::is_elevated()
}

#[tauri::command]
fn relaunch_admin(app: tauri::AppHandle) -> Result<(), String> {
    elevation::relaunch_as_admin()?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
async fn list_programs() -> Result<Vec<programs::Program>, String> {
    tauri::async_runtime::spawn_blocking(programs::list).await.map_err(|e| e.to_string())
}

/// Taille réelle des dossiers d'installation (peut prendre quelques secondes).
#[tauri::command]
async fn program_sizes() -> Result<Vec<programs::RealSize>, String> {
    tauri::async_runtime::spawn_blocking(|| programs::real_sizes(&programs::list()))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn uninstall_program(id: String) -> Result<(), String> {
    programs::uninstall(&id, &journal::data_dir())
}

#[tauri::command]
fn save_report(content: String) -> Result<String, String> {
    actions::save_report(&content, &journal::data_dir()).map(|p| p.display().to_string())
}

/// Autorise la fenêtre à lire ce seul fichier (photo ou vidéo) pour l'afficher. Rien d'autre n'est ouvert.
#[tauri::command]
fn preview_media(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let file = actions::check_media(Path::new(&path))?;
    app.asset_protocol_scope().allow_file(&file).map_err(|e| e.to_string())?;
    Ok(file.display().to_string())
}

#[tauri::command]
fn open_media(path: String) -> Result<(), String> {
    actions::open_media(Path::new(&path))
}

/// Verdict « qu'est-ce que c'est / est-ce que ça casse » pour un lot de chemins. Lecture seule.
#[tauri::command]
async fn explain_paths(paths: Vec<String>) -> Result<Vec<knowledge::Explanation>, String> {
    tauri::async_runtime::spawn_blocking(move || knowledge::explain_all(&paths)).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn list_drives() -> Vec<scan::DriveInfo> {
    scan::drives()
}

#[derive(Serialize, Clone)]
struct DupProgress {
    done: u64,
    total: u64,
}

/// Cherche les doublons parmi les fichiers de la dernière analyse. Événements `dup-progress`.
#[tauri::command]
async fn find_duplicates(app: tauri::AppHandle, state: State<'_, ScanState>) -> Result<Vec<duplicates::DupGroup>, String> {
    let candidates = state.dups.lock().unwrap().clone();
    if candidates.is_empty() {
        return Err("Run a scan first".into());
    }
    state.dup_cancel.store(false, Relaxed);
    let cancel = state.dup_cancel.clone();
    tauri::async_runtime::spawn_blocking(move || {
        duplicates::find(candidates, &cancel, &|done, total| {
            let _ = app.emit("dup-progress", DupProgress { done, total });
        })
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn cancel_duplicates(state: State<ScanState>) {
    state.dup_cancel.store(true, Relaxed);
}

/// Supprime des dossiers régénérables de projets. Seuls ceux trouvés par la dernière analyse sont acceptés.
#[tauri::command]
async fn delete_artifacts(paths: Vec<String>, state: State<'_, ScanState>) -> Result<catalog::CleanReport, String> {
    let known = state.artifacts.lock().unwrap().clone();
    if let Some(p) = paths.iter().find(|p| !known.contains(p)) {
        return Err(format!("Refused: {p} was not found by the last scan"));
    }
    let dirs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    tauri::async_runtime::spawn_blocking(move || catalog::delete_project_dirs(&dirs, &journal::data_dir()))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
struct PreviewEntry {
    id: String,
    files: u64,
    bytes: u64,
    sample: Vec<scan::Item>,
}

#[derive(Serialize)]
struct PreviewView {
    entries: Vec<PreviewEntry>,
    /// Liste complète enregistrée dans les rapports de Kleaner.
    file: String,
}

/// Simulation : ce que les nettoyages choisis supprimeraient, fichier par fichier. Ne supprime rien.
#[tauri::command]
async fn preview_cleaners(ids: Vec<String>) -> Result<PreviewView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut text = String::new();
        let mut entries = Vec::new();
        for id in &ids {
            let Some(c) = catalog::get(id) else { continue };
            let (r, list) = catalog::preview(c);
            text.push_str(&format!("## {} — {} files, {} bytes\n", c.label.en, r.files, r.bytes));
            if let Some(cmd) = &c.command {
                text.push_str(&format!("command: {cmd}\n"));
            }
            for (p, size) in &list {
                text.push_str(&format!("{size}\t{p}\n"));
            }
            text.push('\n');
            let sample = list.iter().take(200).map(|(p, size)| scan::Item { size: *size, path: p.clone(), modified: 0 }).collect();
            entries.push(PreviewEntry { id: id.clone(), files: r.files, bytes: r.bytes, sample });
        }
        let dir = journal::data_dir().join("rapports");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let file = dir.join(format!("cleanup-preview-{}.txt", journal::Entry::new("x").ts));
        std::fs::write(&file, text).map_err(|e| e.to_string())?;
        Ok(PreviewView { entries, file: file.display().to_string() })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Restes de programmes désinstallés. Lecture seule ; la liste est gardée pour valider la suppression.
#[tauri::command]
async fn find_leftovers(state: State<'_, ScanState>) -> Result<Vec<leftovers::Leftover>, String> {
    let list = tauri::async_runtime::spawn_blocking(|| leftovers::find(&programs::list())).await.map_err(|e| e.to_string())?;
    *state.leftovers.lock().unwrap() = list.clone();
    Ok(list)
}

/// Envoie à la Corbeille des restes trouvés par la dernière recherche (et seulement ceux-là).
#[tauri::command]
async fn trash_leftovers(paths: Vec<String>, state: State<'_, ScanState>) -> Result<actions::ActionReport, String> {
    let known = state.leftovers.lock().unwrap().clone();
    let mut sizes = Vec::new();
    for p in &paths {
        match known.iter().find(|l| &l.path == p) {
            Some(l) if l.trashable => sizes.push(l.size),
            Some(_) => return Err(format!("Refused: {p} needs admin rights, delete it from Explorer")),
            None => return Err(format!("Refused: {p} was not found by the last search")),
        }
    }
    let dirs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    tauri::async_runtime::spawn_blocking(move || actions::trash_dirs(&dirs, &guard::Protected::from_env(), &journal::data_dir(), &sizes))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if !elevation::single_instance() {
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ScanState::default())
        .invoke_handler(tauri::generate_handler![
            read_journal,
            remove_kleaner_data,
            start_scan,
            cancel_scan,
            move_to_trash,
            list_cleaners,
            estimate_cleaners,
            run_cleaners,
            relaunch_admin,
            list_programs,
            uninstall_program,
            save_report,
            program_sizes,
            is_elevated,
            preview_media,
            open_media,
            explain_paths,
            list_drives,
            find_duplicates,
            cancel_duplicates,
            delete_artifacts,
            preview_cleaners,
            find_leftovers,
            trash_leftovers
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
