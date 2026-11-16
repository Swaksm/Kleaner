//! Démarrage avec Windows, sur demande seulement : une tâche planifiée « Kleaner » lance l'exe réduit
//! (`--minimized`) à l'ouverture de session de l'utilisateur ; version installée : avec les droits admin, sans invite UAC.
//! Kleaner n'analyse ni ne nettoie rien au démarrage. Créer ou retirer la tâche demande les droits admin.
//! Version Store : la tâche de démarrage du paquet MSIX (`StartupTask`), sans droits admin ; la tâche planifiée n'y sert pas.

use crate::journal;
use crate::store;
use crate::system::ps_run;
use std::path::Path;

pub const MINIMIZED_ARG: &str = "--minimized";
const TASK: &str = "Kleaner";

/// Chaîne entre apostrophes pour PowerShell.
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Script qui crée (ou remplace) la tâche pour l'utilisateur courant.
/// `admin` : droits admin sans invite, seulement pour l'exe installé dans Program Files (qu'un admin seul peut remplacer).
/// L'exe portable (Téléchargements, Bureau…) peut être écrasé par n'importe quel programme : sa tâche démarre sans droits admin.
fn create_script(exe: &Path, admin: bool) -> String {
    format!(
        "$user = \"$env:USERDOMAIN\\$env:USERNAME\";
        $action = New-ScheduledTaskAction -Execute {exe} -Argument {arg};
        $trigger = New-ScheduledTaskTrigger -AtLogOn -User $user;
        $principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel {level};
        $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit 0;
        Register-ScheduledTask -TaskName {task} -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force | Out-Null",
        exe = quote(&exe.display().to_string()),
        arg = quote(MINIMIZED_ARG),
        task = quote(TASK),
        level = if admin { "Highest" } else { "Limited" },
    )
}

/// `None` : pas de tâche ; `Some(false)` : tâche désactivée (page Démarrage), elle ne lance plus rien.
fn state() -> Option<bool> {
    let script = format!("$t = Get-ScheduledTask -TaskName {} -ErrorAction SilentlyContinue; if ($t) {{ $t.State }}", quote(TASK));
    let (out, ..) = ps_run(&script).ok()?;
    (!out.is_empty()).then(|| out != "Disabled")
}

/// Tâche de démarrage du paquet (version Store). Windows la crée à l'installation, désactivée.
fn store_task() -> windows::core::Result<windows::ApplicationModel::StartupTask> {
    windows::ApplicationModel::StartupTask::GetAsync(&store::STARTUP_TASK.into())?.join()
}

fn store_enabled() -> bool {
    use windows::ApplicationModel::StartupTaskState as S;
    store_task().and_then(|t| t.State()).is_ok_and(|s| s == S::Enabled || s == S::EnabledByPolicy)
}

pub fn enabled() -> bool {
    if store::is_packaged() { store_enabled() } else { state() == Some(true) }
}

/// La tâche existe, active ou non. Version Store : seulement si elle est active (le paquet la garde toujours).
pub fn exists() -> bool {
    if store::is_packaged() { store_enabled() } else { state().is_some() }
}

/// Version Store. Si l'utilisateur l'a coupée dans les Paramètres de Windows, seul lui peut la rallumer là-bas.
fn set_store(on: bool) -> Result<(), String> {
    use windows::ApplicationModel::StartupTaskState as S;
    let task = store_task().map_err(|e| e.to_string())?;
    if !on {
        return task.Disable().map_err(|e| e.to_string());
    }
    match task.RequestEnableAsync().and_then(|a| a.join()).map_err(|e| e.to_string())? {
        S::Enabled | S::EnabledByPolicy => Ok(()),
        S::DisabledByUser => Err("Startup was turned off in Windows Settings (Apps > Startup): turn Kleaner on there.".into()),
        _ => Err("Startup is blocked on this PC (policy).".into()),
    }
}

/// Active ou retire le démarrage automatique. Écrit une ligne dans le journal.
pub fn set(on: bool, journal_dir: &Path) -> Result<(), String> {
    if store::is_packaged() {
        let result = set_store(on);
        let mut entry = journal::Entry::new(if on { "autostart-on" } else { "autostart-off" });
        entry.paths = vec![format!(r"Startup task\{}", store::STARTUP_TASK)];
        entry.error = result.as_ref().err().cloned();
        journal::append(journal_dir, &entry).map_err(|e| format!("Log: {e}"))?;
        return result;
    }
    if !crate::elevation::is_elevated() {
        return Err("Admin rights are needed to change the startup task.".into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let script = if on {
        create_script(&exe, crate::updater::is_installed())
    } else {
        format!("Unregister-ScheduledTask -TaskName {} -Confirm:$false -ErrorAction SilentlyContinue", quote(TASK))
    };
    let (_, err, ok) = ps_run(&script)?;
    let mut entry = journal::Entry::new(if on { "autostart-on" } else { "autostart-off" });
    entry.paths = vec![format!(r"Task Scheduler\{TASK}")];
    if !ok {
        entry.error = Some(err.clone());
    }
    journal::append(journal_dir, &entry).map_err(|e| format!("Log: {e}"))?;
    if ok { Ok(()) } else { Err(err) }
}

/// Exe portable dont la tâche a été créée avec les droits admin (versions d'avant la 1.3) : recréée sans droits admin.
/// Possible seulement quand Kleaner tourne en admin, ce que fait justement cette tâche à l'ouverture de session.
/// Une tâche désactivée (page Démarrage) n'est pas touchée. Journalisé.
pub fn downgrade_portable(journal_dir: &Path) {
    if store::is_packaged() || crate::updater::is_installed() || !crate::elevation::is_elevated() {
        return;
    }
    let script = format!("$t = Get-ScheduledTask -TaskName {} -ErrorAction SilentlyContinue; if ($t) {{ \"$($t.Principal.RunLevel)|$($t.State)\" }}", quote(TASK));
    let admin_task = ps_run(&script).is_ok_and(|(out, ..)| out.starts_with("Highest|") && !out.ends_with("|Disabled"));
    let Ok(exe) = std::env::current_exe() else { return };
    if !admin_task {
        return;
    }
    let Ok((_, err, ok)) = ps_run(&create_script(&exe, false)) else { return };
    let mut entry = journal::Entry::new("autostart-limited");
    entry.paths = vec![format!(r"Task Scheduler\{TASK}")];
    entry.error = (!ok).then_some(err);
    let _ = journal::append(journal_dir, &entry);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_quotes_the_path_and_runs_minimized_as_admin() {
        let s = create_script(Path::new(r"C:\Program Files\Kleaner\it's.exe"), true);
        assert!(s.contains(r"-Execute 'C:\Program Files\Kleaner\it''s.exe'"), "{s}");
        assert!(s.contains("-Argument '--minimized'"));
        assert!(s.contains("-RunLevel Highest"));
        assert!(s.contains("-AtLogOn -User $user"), "seulement pour l'utilisateur courant");
        let portable = create_script(Path::new(r"C:\Users\me\Downloads\kleaner.exe"), false);
        assert!(portable.contains("-RunLevel Limited"), "exe portable : jamais admin sans invite");
    }
}
