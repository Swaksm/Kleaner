//! Mode « démarrage avec Windows » (`--minimized`) : fenêtre cachée, icône près de l'horloge (Ouvrir / Quitter)
//! et rappel discret quand un disque fixe a moins de 10 % d'espace libre. Au plus une fois par mois, rappel de l'espace
//! que les nettoyages sûrs libéreraient (simple estimation). Rien n'est nettoyé tant que l'utilisateur n'ouvre pas Kleaner.

use crate::catalog::{self, Level};
use crate::scan::{self, DriveKind};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

const CHECK_EVERY: Duration = Duration::from_secs(30 * 60);
const REMIND_AGAIN: Duration = Duration::from_secs(24 * 3600);
const MONTHLY: Duration = Duration::from_secs(30 * 24 * 3600);
/// En dessous, le rappel mensuel ne dérange personne.
const WORTH_IT: u64 = 1_000_000_000;

/// Textes de l'icône et du rappel, dans la langue de Windows (la langue choisie dans Kleaner vit dans la WebView).
struct Texts {
    open: &'static str,
    quit: &'static str,
    title: &'static str,
    /// « {drive} » et « {free} » remplacés.
    body: &'static str,
    safe_title: &'static str,
    /// « {size} » remplacé.
    safe_body: &'static str,
}

fn texts() -> Texts {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    match unsafe { GetUserDefaultUILanguage() } & 0x3ff {
        0x0c => Texts { open: "Ouvrir Kleaner", quit: "Quitter", title: "Disque presque plein", body: "{drive} : plus que {free} de libre. Ouvre Kleaner pour voir ce qui peut partir.", safe_title: "Place à libérer", safe_body: "{size} peuvent partir sans risque (caches, fichiers temporaires). Ouvre Kleaner pour choisir : rien n'est supprimé sans toi." },
        0x0a => Texts { open: "Abrir Kleaner", quit: "Salir", title: "Disco casi lleno", body: "{drive}: solo quedan {free} libres. Abre Kleaner para ver qué se puede quitar.", safe_title: "Espacio para liberar", safe_body: "{size} se pueden quitar sin riesgo (cachés, archivos temporales). Abre Kleaner para elegir: no se borra nada sin ti." },
        0x07 => Texts { open: "Kleaner öffnen", quit: "Beenden", title: "Laufwerk fast voll", body: "{drive}: nur noch {free} frei. Öffne Kleaner, um zu sehen, was weg kann.", safe_title: "Platz zum Freigeben", safe_body: "{size} können gefahrlos weg (Caches, temporäre Dateien). Öffne Kleaner zum Auswählen: Ohne dich wird nichts gelöscht." },
        0x10 => Texts { open: "Apri Kleaner", quit: "Esci", title: "Disco quasi pieno", body: "{drive}: restano solo {free} liberi. Apri Kleaner per vedere cosa si può togliere.", safe_title: "Spazio da liberare", safe_body: "{size} possono andare via senza rischi (cache, file temporanei). Apri Kleaner per scegliere: nulla viene eliminato senza di te." },
        0x16 => Texts { open: "Abrir o Kleaner", quit: "Sair", title: "Disco quase cheio", body: "{drive}: só restam {free} livres. Abra o Kleaner para ver o que pode sair.", safe_title: "Espaço para liberar", safe_body: "{size} podem sair sem risco (caches, arquivos temporários). Abra o Kleaner para escolher: nada é apagado sem você." },
        0x19 => Texts { open: "Открыть Kleaner", quit: "Выйти", title: "Диск почти заполнен", body: "{drive}: свободно всего {free}. Откройте Kleaner, чтобы узнать, что можно удалить.", safe_title: "Можно освободить место", safe_body: "{size} можно удалить без риска (кэши, временные файлы). Откройте Kleaner, чтобы выбрать: без вас ничего не удаляется." },
        0x04 => Texts { open: "打开 Kleaner", quit: "退出", title: "磁盘快满了", body: "{drive} 只剩 {free} 可用空间。打开 Kleaner 看看哪些可以清理。", safe_title: "可释放空间", safe_body: "{size} 可以安全清理（缓存、临时文件）。打开 Kleaner 自行选择：未经你同意不会删除任何内容。" },
        _ => Texts { open: "Open Kleaner", quit: "Quit", title: "Drive almost full", body: "{drive} has only {free} free. Open Kleaner to see what can go.", safe_title: "Space to free", safe_body: "{size} can go safely (caches, temporary files). Open Kleaner to choose: nothing is deleted without you." },
    }
}

/// Disque « presque plein » : moins de 10 % libre.
pub fn almost_full(free: u64, total: u64) -> bool {
    total > 0 && free.saturating_mul(10) < total
}

fn gib(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1e9)
}

/// Ce que les nettoyages sûrs sans admin libéreraient maintenant (estimation, rien n'est supprimé).
/// Même règle que le « sans risque » de l'Accueil (`useRecoverable`) : catalogue communautaire exclu.
fn safe_bytes() -> u64 {
    catalog::all()
        .iter()
        .filter(|c| c.level == Level::Safe && !c.admin && c.group != "community")
        .map(|c| catalog::estimate(c).bytes)
        .sum()
}

/// Le fichier `reminder` du dossier de données date le dernier rappel mensuel (absent : jamais).
fn monthly_due(file: &Path) -> bool {
    std::fs::metadata(file).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_none_or(|e| e >= MONTHLY)
}

/// Rappel mensuel coupé dans les Paramètres : fichier `no-reminder` du dossier de données (absent : actif).
pub fn reminder_enabled() -> bool {
    !crate::journal::data_dir().join("no-reminder").exists()
}

pub fn set_reminder(on: bool) -> std::io::Result<()> {
    let dir = crate::journal::data_dir();
    let file = dir.join("no-reminder");
    if on {
        std::fs::remove_file(&file).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) })
    } else {
        std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, b""))
    }
}

fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn start(app: &AppHandle) -> tauri::Result<()> {
    let tx = texts();
    let open = MenuItem::with_id(app, "open", tx.open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", tx.quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut tray = TrayIconBuilder::with_id("kleaner")
        .tooltip("Kleaner")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id().as_ref() {
            "open" => show(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    // Fermer la fenêtre la range près de l'horloge ; « Quitter » ferme Kleaner.
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
        let hidden = w.clone();
        w.on_window_event(move |e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                api.prevent_close();
                let _ = hidden.hide();
            }
        });
    }

    let app = app.clone();
    std::thread::spawn(move || {
        let mut last: HashMap<String, Instant> = HashMap::new();
        loop {
            for d in scan::drives().into_iter().filter(|d| d.kind == DriveKind::Fixed && almost_full(d.free, d.total)) {
                if last.get(&d.root).is_some_and(|t| t.elapsed() < REMIND_AGAIN) {
                    continue;
                }
                last.insert(d.root.clone(), Instant::now());
                let body = tx.body.replace("{drive}", d.root.trim_end_matches('\\')).replace("{free}", &gib(d.free));
                let _ = app.notification().builder().title(tx.title).body(body).show();
            }
            std::thread::sleep(CHECK_EVERY);
            // Après la première attente : pas de rappel dès l'ouverture de session.
            let dir = crate::journal::data_dir();
            let file = dir.join("reminder");
            if reminder_enabled() && monthly_due(&file) {
                let bytes = safe_bytes();
                // Daté même sous le seuil : on ne réestime pas toutes les 30 min.
                let _ = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, b""));
                if bytes >= WORTH_IT {
                    let body = tx.safe_body.replace("{size}", &gib(bytes));
                    let _ = app.notification().builder().title(tx.safe_title).body(body).show();
                }
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{almost_full, monthly_due};

    #[test]
    fn under_ten_percent_is_almost_full() {
        assert!(almost_full(9, 100));
        assert!(!almost_full(10, 100));
        assert!(!almost_full(0, 0), "disque vide ou illisible");
    }

    #[test]
    fn monthly_reminder_waits_a_month() {
        let file = std::env::temp_dir().join(format!("kleaner-reminder-{}", std::process::id()));
        let _ = std::fs::remove_file(&file);
        assert!(monthly_due(&file), "jamais rappelé");
        std::fs::write(&file, b"").unwrap();
        assert!(!monthly_due(&file), "rappelé à l'instant");
        std::fs::remove_file(&file).unwrap();
    }
}
