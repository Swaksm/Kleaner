//! Mode « démarrage avec Windows » (`--minimized`) : fenêtre cachée, icône près de l'horloge (Ouvrir / Quitter)
//! et rappel discret quand un disque fixe a moins de 10 % d'espace libre. Lecture seule : rien n'est analysé
//! ni nettoyé tant que l'utilisateur n'ouvre pas Kleaner.

use crate::scan::{self, DriveKind};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

const CHECK_EVERY: Duration = Duration::from_secs(30 * 60);
const REMIND_AGAIN: Duration = Duration::from_secs(24 * 3600);

/// Textes de l'icône et du rappel, dans la langue de Windows (la langue choisie dans Kleaner vit dans la WebView).
struct Texts {
    open: &'static str,
    quit: &'static str,
    title: &'static str,
    /// « {drive} » et « {free} » remplacés.
    body: &'static str,
}

fn texts() -> Texts {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    match unsafe { GetUserDefaultUILanguage() } & 0x3ff {
        0x0c => Texts { open: "Ouvrir Kleaner", quit: "Quitter", title: "Disque presque plein", body: "{drive} : plus que {free} de libre. Ouvre Kleaner pour voir ce qui peut partir." },
        0x0a => Texts { open: "Abrir Kleaner", quit: "Salir", title: "Disco casi lleno", body: "{drive}: solo quedan {free} libres. Abre Kleaner para ver qué se puede quitar." },
        0x07 => Texts { open: "Kleaner öffnen", quit: "Beenden", title: "Laufwerk fast voll", body: "{drive}: nur noch {free} frei. Öffne Kleaner, um zu sehen, was weg kann." },
        0x10 => Texts { open: "Apri Kleaner", quit: "Esci", title: "Disco quasi pieno", body: "{drive}: restano solo {free} liberi. Apri Kleaner per vedere cosa si può togliere." },
        0x16 => Texts { open: "Abrir o Kleaner", quit: "Sair", title: "Disco quase cheio", body: "{drive}: só restam {free} livres. Abra o Kleaner para ver o que pode sair." },
        0x19 => Texts { open: "Открыть Kleaner", quit: "Выйти", title: "Диск почти заполнен", body: "{drive}: свободно всего {free}. Откройте Kleaner, чтобы узнать, что можно удалить." },
        0x04 => Texts { open: "打开 Kleaner", quit: "退出", title: "磁盘快满了", body: "{drive} 只剩 {free} 可用空间。打开 Kleaner 看看哪些可以清理。" },
        _ => Texts { open: "Open Kleaner", quit: "Quit", title: "Drive almost full", body: "{drive} has only {free} free. Open Kleaner to see what can go." },
    }
}

/// Disque « presque plein » : moins de 10 % libre.
pub fn almost_full(free: u64, total: u64) -> bool {
    total > 0 && free.saturating_mul(10) < total
}

fn gib(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1e9)
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
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::almost_full;

    #[test]
    fn under_ten_percent_is_almost_full() {
        assert!(almost_full(9, 100));
        assert!(!almost_full(10, 100));
        assert!(!almost_full(0, 0), "disque vide ou illisible");
    }
}
