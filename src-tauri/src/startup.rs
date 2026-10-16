//! Programmes lancés à l'ouverture de session : clés Run, dossiers Démarrage, tâches planifiées, services.
//! Désactiver ne supprime rien : comme le Gestionnaire des tâches, on écrit l'état dans `StartupApproved`
//! (clés et dossiers) ou on désactive la tâche. On peut toujours réactiver.

use crate::journal;
use crate::system::{ps_json, ps_run};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, REG_BINARY};
use winreg::{RegKey, RegValue};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";
const STARTUP_DIR: &str = r"Microsoft\Windows\Start Menu\Programs\Startup";

/// (étiquette dans l'id, ruche, clé Run, sous-clé StartupApproved, machine entière)
const REG_SOURCES: [(&str, winreg::HKEY, &str, &str, bool); 3] = [
    ("HKCU", HKEY_CURRENT_USER, RUN, "Run", false),
    ("HKLM", HKEY_LOCAL_MACHINE, RUN, "Run", true),
    ("HKLM32", HKEY_LOCAL_MACHINE, RUN32, "Run32", true),
];

#[derive(Serialize, Clone, Debug)]
pub struct StartupItem {
    /// `reg|HKCU|Nom`, `folder|user|fichier`, `task|\chemin\|Nom`, `svc|nom`.
    pub id: String,
    pub name: String,
    pub command: String,
    /// registry, folder, task ou service.
    pub source: &'static str,
    /// Tous les utilisateurs (droits admin pour modifier) ou le compte actuel.
    pub machine: bool,
    pub enabled: bool,
    /// Services : lus seulement.
    pub can_toggle: bool,
    /// Windows, sécurité ou pilote : mieux vaut le laisser.
    pub keep: bool,
    /// Durée de ralentissement au dernier démarrage (Windows l'a signalé), en ms ; 0 si inconnu.
    pub slow_ms: u64,
    /// Le programme que cette entrée lance n'existe plus : elle ne fait plus rien.
    pub missing: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all(deserialize = "PascalCase"), default)]
pub struct Boot {
    /// Secondes Unix.
    pub time: u64,
    pub boot_ms: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct StartupView {
    pub items: Vec<StartupItem>,
    pub boots: Vec<Boot>,
}

/// `StartupApproved` : 12 octets. Premier octet pair (02, 06) = activé, impair (03) = désactivé + date.
pub fn approved_bytes(enabled: bool, unix_secs: u64) -> Vec<u8> {
    let mut b = vec![0u8; 12];
    if enabled {
        b[0] = 2;
    } else {
        b[0] = 3;
        let filetime = (unix_secs + 11_644_473_600) * 10_000_000;
        b[4..].copy_from_slice(&filetime.to_le_bytes());
    }
    b
}

pub fn is_enabled(bytes: &[u8]) -> bool {
    bytes.first().is_none_or(|b| b & 1 == 0)
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn write_approved(root: &RegKey, sub: &str, name: &str, enabled: bool) -> std::io::Result<()> {
    let (k, _) = root.create_subkey_with_flags(format!(r"{APPROVED}\{sub}"), KEY_SET_VALUE | KEY_READ)?;
    k.set_raw_value(name, &RegValue { bytes: approved_bytes(enabled, now()).into(), vtype: REG_BINARY })
}

fn read_approved(root: &RegKey, sub: &str, name: &str) -> bool {
    root.open_subkey(format!(r"{APPROVED}\{sub}"))
        .and_then(|k| k.get_raw_value(name))
        .map_or(true, |v| is_enabled(&v.bytes))
}

/// Windows, sécurité et pilotes : on ne les propose pas comme « à désactiver ».
fn keep(name: &str, command: &str) -> bool {
    let (n, c) = (name.to_lowercase(), command.to_lowercase());
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into()).to_lowercase();
    c.contains(&format!("{windir}\\")) && !c.contains("\\temp\\")
        || n.starts_with("securityhealth")
        || c.contains(r"\windows defender\")
        || ["realtek", "synaptics", "elantech", "nvidia", "igfx", "intel(r)", "waves"].iter().any(|v| n.contains(v) || c.contains(&format!("\\{v}")))
}

fn exe_name(command: &str) -> String {
    let exe = crate::programs::split_command(command).map(|(e, _)| e).unwrap_or_default();
    Path::new(&exe).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default()
}

fn startup_dir(machine: bool) -> Option<PathBuf> {
    let base = std::env::var_os(if machine { "ProgramData" } else { "APPDATA" })?;
    Some(PathBuf::from(base).join(STARTUP_DIR))
}

fn registry_items() -> Vec<StartupItem> {
    let mut out = Vec::new();
    for (label, hive, run, approved, machine) in REG_SOURCES {
        let root = RegKey::predef(hive);
        let Ok(key) = root.open_subkey_with_flags(run, KEY_READ) else { continue };
        for (name, _) in key.enum_values().flatten() {
            let command = key.get_value::<String, _>(&name).unwrap_or_default();
            out.push(StartupItem {
                id: format!("reg|{label}|{name}"),
                enabled: read_approved(&root, approved, &name),
                keep: keep(&name, &command),
                name,
                command,
                source: "registry",
                machine,
                can_toggle: true,
                slow_ms: 0,
                missing: false,
            });
        }
    }
    out
}

fn folder_items() -> Vec<StartupItem> {
    let mut out = Vec::new();
    for machine in [false, true] {
        let Some(dir) = startup_dir(machine) else { continue };
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let root = RegKey::predef(if machine { HKEY_LOCAL_MACHINE } else { HKEY_CURRENT_USER });
        for e in rd.flatten() {
            let file = e.file_name().to_string_lossy().to_string();
            if file.eq_ignore_ascii_case("desktop.ini") {
                continue;
            }
            let command = e.path().display().to_string();
            out.push(StartupItem {
                id: format!("folder|{}|{file}", if machine { "machine" } else { "user" }),
                enabled: read_approved(&root, "StartupFolder", &file),
                keep: keep(&file, &command),
                name: file.trim_end_matches(".lnk").to_string(),
                command,
                source: "folder",
                machine,
                can_toggle: true,
                slow_ms: 0,
                missing: false,
            });
        }
    }
    out
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct TaskRow {
    name: String,
    path: String,
    enabled: bool,
    command: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct SvcRow {
    name: String,
    display: String,
    command: String,
    running: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct SlowRow {
    name: String,
    ms: u64,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct Diag {
    boots: Vec<Boot>,
    slow: Vec<SlowRow>,
}

fn task_items() -> Vec<StartupItem> {
    ps_json::<Vec<TaskRow>>(
        r#"ConvertTo-Json -Compress -InputObject @(Get-ScheduledTask -ErrorAction SilentlyContinue | Where-Object {
          $_.TaskPath -notlike '\Microsoft\*' -and ($_.Triggers | Where-Object { $_.CimClass.CimClassName -in 'MSFT_TaskLogonTrigger','MSFT_TaskBootTrigger' }) } |
          ForEach-Object { $a = $_.Actions | Select-Object -First 1;
            [pscustomobject]@{ Name = $_.TaskName; Path = $_.TaskPath; Enabled = ($_.State -ne 'Disabled'); Command = ("$($a.Execute) $($a.Arguments)").Trim() } })"#,
    )
    .unwrap_or_default()
    .into_iter()
    .map(|t| StartupItem {
        id: format!("task|{}|{}", t.path, t.name),
        keep: keep(&t.name, &t.command),
        name: t.name,
        command: t.command,
        source: "task",
        machine: false,
        enabled: t.enabled,
        can_toggle: true,
        slow_ms: 0,
        missing: false,
    })
    .collect()
}

fn service_items() -> Vec<StartupItem> {
    ps_json::<Vec<SvcRow>>(
        r#"ConvertTo-Json -Compress -InputObject @(Get-CimInstance Win32_Service -Filter "StartMode='Auto'" -ErrorAction SilentlyContinue |
          Where-Object { $_.PathName -and $_.PathName -notlike '*\Windows\*' -and $_.PathName -notlike '*\Windows Defender\*' } |
          ForEach-Object { [pscustomobject]@{ Name = $_.Name; Display = $_.DisplayName; Command = $_.PathName; Running = ($_.State -eq 'Running') } })"#,
    )
    .unwrap_or_default()
    .into_iter()
    .map(|s| StartupItem {
        id: format!("svc|{}", s.name),
        keep: keep(&s.display, &s.command),
        name: if s.display.is_empty() { s.name } else { s.display },
        command: s.command,
        source: "service",
        machine: true,
        enabled: s.running,
        can_toggle: false,
        slow_ms: 0,
        missing: false,
    })
    .collect()
}

/// Durées de démarrage et applications signalées lentes par Windows (journal Diagnostics-Performance).
fn diagnostics() -> Diag {
    ps_json(
        r#"$log = 'Microsoft-Windows-Diagnostics-Performance/Operational';
        function Rows($id, $max) { Get-WinEvent -FilterHashtable @{ LogName = $log; Id = $id } -MaxEvents $max -ErrorAction SilentlyContinue |
          ForEach-Object { $d = @{}; ([xml]$_.ToXml()).Event.EventData.Data | ForEach-Object { $d[$_.Name] = $_.'#text' };
            [pscustomobject]@{ T = [DateTimeOffset]::new($_.TimeCreated).ToUnixTimeSeconds(); D = $d } } }
        $boots = @(Rows 100 8 | ForEach-Object { [pscustomobject]@{ Time = $_.T; BootMs = [int64]$_.D['BootTime'] } });
        $slow = @(Rows 101 200 | ForEach-Object { [pscustomobject]@{ Name = "$($_.D['Name'])"; Ms = [int64]$_.D['TotalTime'] } });
        ConvertTo-Json -Compress -Depth 4 -InputObject ([pscustomobject]@{ Boots = $boots; Slow = $slow })"#,
    )
    .unwrap_or_default()
}

pub fn list() -> StartupView {
    let (diag, tasks, services) = std::thread::scope(|s| {
        let d = s.spawn(diagnostics);
        let t = s.spawn(task_items);
        let v = s.spawn(service_items);
        (d.join().unwrap_or_default(), t.join().unwrap_or_default(), v.join().unwrap_or_default())
    });
    let mut items: Vec<StartupItem> = registry_items();
    items.extend(folder_items());
    items.extend(tasks);
    items.extend(services);
    for it in &mut items {
        it.missing = crate::programs::exe_missing(&it.command);
        let exe = exe_name(&it.command);
        if !exe.is_empty() {
            it.slow_ms = diag.slow.iter().filter(|s| s.name.to_lowercase() == exe).map(|s| s.ms).max().unwrap_or(0);
        }
    }
    // Les activés d'abord, les plus lents en tête.
    items.sort_by(|a, b| b.enabled.cmp(&a.enabled).then(b.slow_ms.cmp(&a.slow_ms)).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    StartupView { items, boots: diag.boots }
}

/// Active ou désactive un élément de la dernière liste. Écrit au journal.
pub fn set_enabled(id: &str, enabled: bool, journal_dir: &Path) -> Result<(), String> {
    let parts: Vec<&str> = id.splitn(3, '|').collect();
    let result = match parts.as_slice() {
        ["reg", label, name] => {
            let (_, hive, _, approved, _) = REG_SOURCES.iter().find(|s| s.0 == *label).ok_or("Unknown source")?;
            write_approved(&RegKey::predef(*hive), approved, name, enabled).map_err(denied)
        }
        ["folder", who, file] => {
            let hive = if *who == "machine" { HKEY_LOCAL_MACHINE } else { HKEY_CURRENT_USER };
            write_approved(&RegKey::predef(hive), "StartupFolder", file, enabled).map_err(denied)
        }
        ["task", path, name] => {
            let verb = if enabled { "Enable" } else { "Disable" };
            let q = |s: &str| s.replace('\'', "''");
            let (_, err, ok) = ps_run(&format!(
                "{verb}-ScheduledTask -TaskPath '{}' -TaskName '{}' -ErrorAction Stop | Out-Null",
                q(path),
                q(name)
            ))?;
            if ok { Ok(()) } else { Err(if err.is_empty() { "Task not changed (admin rights may be needed)".into() } else { err }) }
        }
        _ => Err("This item can only be changed in Windows".into()),
    };
    let mut j = journal::Entry::new(if enabled { "startup-enable" } else { "startup-disable" });
    j.paths = vec![id.to_string()];
    j.files = u64::from(result.is_ok());
    j.error = result.as_ref().err().cloned();
    journal::append(journal_dir, &j).map_err(|e| format!("Log: {e}"))?;
    result
}

fn denied(e: std::io::Error) -> String {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        "Access denied: this item is for all users, relaunch Kleaner as administrator".into()
    } else {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_bytes_match_task_manager_format() {
        let on = approved_bytes(true, 0);
        assert_eq!((on.len(), on[0]), (12, 2));
        assert!(is_enabled(&on));
        let off = approved_bytes(false, 1_700_000_000);
        assert_eq!((off.len(), off[0]), (12, 3));
        assert!(!is_enabled(&off));
        assert!(off[4..].iter().any(|b| *b != 0), "date de désactivation");
        assert!(is_enabled(&[6, 0, 0, 0]) && !is_enabled(&[7, 0, 0, 0]));
    }

    #[test]
    fn toggle_round_trip_in_a_test_key() {
        let root = RegKey::predef(HKEY_CURRENT_USER);
        let sub = format!("Kleaner-Test-{}", std::process::id());
        let name = "App";
        let base = format!(r"{APPROVED}\{sub}");
        assert!(read_approved(&root, &sub, name), "absent = activé");
        write_approved(&root, &sub, name, false).unwrap();
        assert!(!read_approved(&root, &sub, name));
        write_approved(&root, &sub, name, true).unwrap();
        assert!(read_approved(&root, &sub, name));
        root.delete_subkey_all(base).unwrap();
    }

    /// Une vraie entrée `Run` de HKCU : désactivée, vue désactivée dans la liste, réactivée, journalisée.
    #[test]
    fn set_enabled_on_a_run_entry() {
        let name = format!("KleanerStartupTest-{}", std::process::id());
        let dir = std::env::temp_dir().join(format!("kleaner-startup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (run, _) = hkcu.create_subkey(RUN).unwrap();
        run.set_value(&name, &r"C:\Kleaner-test\app.exe --x").unwrap();
        let id = format!("reg|HKCU|{name}");
        let state = || registry_items().into_iter().find(|i| i.id == id).map(|i| i.enabled);

        assert_eq!(state(), Some(true));
        set_enabled(&id, false, &dir).unwrap();
        assert_eq!(state(), Some(false));
        set_enabled(&id, true, &dir).unwrap();
        assert_eq!(state(), Some(true));
        assert!(set_enabled("svc|Spooler", false, &dir).is_err(), "les services ne se changent pas ici");
        let log = std::fs::read_to_string(dir.join("journal.log")).unwrap();
        assert!(log.contains("startup-disable") && log.contains("startup-enable"));

        run.delete_value(&name).unwrap();
        hkcu.open_subkey_with_flags(format!(r"{APPROVED}\Run"), KEY_SET_VALUE).and_then(|k| k.delete_value(&name)).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn windows_and_drivers_are_marked_keep() {
        assert!(keep("SecurityHealth", r"C:\Windows\system32\SecurityHealthSystray.exe"));
        assert!(keep("RtkAudUService", r#""C:\Program Files\Realtek\Audio\RtkAudUService64.exe" -background"#));
        assert!(!keep("Discord", r"C:\Users\me\AppData\Local\Discord\Update.exe --processStart Discord.exe"));
    }

    #[test]
    fn exe_name_from_commands() {
        assert_eq!(exe_name(r#""C:\Program Files\Foo\Foo.exe" --minimized"#), "foo.exe");
        assert_eq!(exe_name(r"C:\Foo\bar.exe /s"), "bar.exe");
    }
}

#[cfg(test)]
mod real_tests {
    /// Sur la vraie machine : `cargo test --release startup::real_tests -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real() {
        let v = super::list();
        for i in &v.items {
            println!("{:9} {:5} {:5} keep={:5} slow={:6} {} | {}", i.source, i.enabled, i.can_toggle, i.keep, i.slow_ms, i.name, i.command);
        }
        println!("boots: {:?}", v.boots);
    }
}
