//! Onglet « Mises à jour et santé » : Windows Update, applications (winget), pilotes graphiques, état de la machine.
//! Kleaner ne télécharge rien lui-même : il lit l'état via les outils de Windows, et les mises à jour passent
//! par Windows Update ou winget, seulement quand l'utilisateur clique.

use crate::journal;
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Lance un script PowerShell : (sortie standard, erreurs, réussite).
pub(crate) fn ps_run(script: &str) -> Result<(String, String, bool), String> {
    let script = format!("$ProgressPreference='SilentlyContinue'; [Console]::OutputEncoding=[Text.Encoding]::UTF8; {script}");
    let out = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    Ok((String::from_utf8_lossy(&out.stdout).trim().to_string(), String::from_utf8_lossy(&out.stderr).trim().to_string(), out.status.success()))
}

/// Lance un script PowerShell qui écrit du JSON (ConvertTo-Json) et le relit.
pub(crate) fn ps_json<T: for<'de> Deserialize<'de>>(script: &str) -> Result<T, String> {
    let (out, err, _) = ps_run(script)?;
    serde_json::from_str(&out).map_err(|e| format!("{e}: {err}"))
}

/// ConvertTo-Json renvoie un objet seul au lieu d'une liste quand il n'y a qu'un élément.
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> From<OneOrMany<T>> for Vec<T> {
    fn from(v: OneOrMany<T>) -> Self {
        match v {
            OneOrMany::One(x) => vec![x],
            OneOrMany::Many(v) => v,
        }
    }
}

// ---------- Machine, système, pilotes graphiques, disques ----------

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all(deserialize = "PascalCase"), default)]
pub struct Gpu {
    pub name: String,
    pub vendor: String,
    pub driver_version: String,
    /// AAAAMMJJ
    pub driver_date: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all(deserialize = "PascalCase"), default)]
pub struct Disk {
    pub name: String,
    pub media: String,
    /// Healthy, Warning, Unhealthy…
    pub health: String,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all(deserialize = "PascalCase"), default)]
pub struct SystemInfo {
    pub os_name: String,
    /// « 24H2 », « 22H2 »…
    pub os_release: String,
    pub os_build: u32,
    pub manufacturer: String,
    pub model: String,
    pub memory: u64,
    pub bios_version: String,
    /// AAAAMMJJ
    pub bios_date: String,
    /// Secondes depuis le dernier démarrage.
    pub uptime: u64,
    #[serde(skip_deserializing)]
    pub support_end: Option<String>,
    #[serde(skip_deserializing)]
    pub gpus: Vec<Gpu>,
    #[serde(skip_deserializing)]
    pub disks: Vec<Disk>,
}

/// Fin du support des éditions Famille et Pro, par numéro de build (dates publiées par Microsoft).
// shortcut: table à compléter à chaque nouvelle version de Windows.
/// Numéro de build de Windows (22000 et plus : Windows 11), 0 si illisible.
pub fn windows_build() -> u32 {
    winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .and_then(|k| k.get_value::<String, _>("CurrentBuildNumber"))
        .ok()
        .and_then(|b| b.parse().ok())
        .unwrap_or(0)
}

pub fn support_end(build: u32) -> Option<&'static str> {
    Some(match build {
        19045 => "2025-10-14", // Windows 10 22H2 (dernière version de Windows 10)
        10240..19045 => "2025-10-14",
        22000 => "2023-10-10", // Windows 11 21H2
        22621 => "2024-10-08", // 22H2
        22631 => "2025-11-11", // 23H2
        26100 => "2026-10-13", // 24H2
        26200 => "2027-10-12", // 25H2
        _ => return None,
    })
}

pub fn info() -> Result<SystemInfo, String> {
    let mut s: SystemInfo = ps_json(
        r#"$os = Get-CimInstance Win32_OperatingSystem; $cs = Get-CimInstance Win32_ComputerSystem; $b = Get-CimInstance Win32_BIOS;
        $rel = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').DisplayVersion;
        [pscustomobject]@{ OsName = $os.Caption; OsRelease = "$rel"; OsBuild = [int]$os.BuildNumber;
          Manufacturer = $cs.Manufacturer; Model = $cs.Model; Memory = [int64]$cs.TotalPhysicalMemory;
          BiosVersion = $b.SMBIOSBIOSVersion; BiosDate = $b.ReleaseDate.ToString('yyyyMMdd');
          Uptime = [int64]((Get-Date) - $os.LastBootUpTime).TotalSeconds } | ConvertTo-Json -Compress"#,
    )?;
    s.support_end = support_end(s.os_build).map(String::from);
    s.gpus = ps_json::<OneOrMany<Gpu>>(
        r#"@(Get-CimInstance Win32_VideoController | Where-Object { $_.DriverVersion } | ForEach-Object {
          [pscustomobject]@{ Name = $_.Name; Vendor = "$($_.AdapterCompatibility)"; DriverVersion = $_.DriverVersion;
            DriverDate = if ($_.DriverDate) { $_.DriverDate.ToString('yyyyMMdd') } else { '' } } }) | ConvertTo-Json -Compress"#,
    )
    .map(Vec::from)
    .unwrap_or_default();
    s.disks = ps_json::<OneOrMany<Disk>>(
        r#"@(Get-PhysicalDisk | ForEach-Object { [pscustomobject]@{ Name = $_.FriendlyName; Media = "$($_.MediaType)";
          Health = "$($_.HealthStatus)"; Size = [int64]$_.Size } }) | ConvertTo-Json -Compress"#,
    )
    .map(Vec::from)
    .unwrap_or_default();
    Ok(s)
}

// ---------- Windows Update (lecture seule) ----------

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all(deserialize = "PascalCase"), default)]
pub struct WinUpdate {
    pub title: String,
    /// « Software » ou « Driver »
    pub kind: String,
    pub size: u64,
    pub mandatory: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all(deserialize = "PascalCase"), default)]
pub struct WinUpdateStatus {
    #[serde(skip_deserializing)]
    pub updates: Vec<WinUpdate>,
    pub reboot_required: bool,
    /// AAAA-MM-JJ, vide si inconnu.
    pub last_check: String,
}

/// Mises à jour proposées par Windows Update (système et pilotes officiels). Lecture seule.
/// La recherche est faite par le service Windows Update (il contacte Microsoft), peut durer une minute.
pub fn windows_updates() -> Result<WinUpdateStatus, String> {
    let mut st: WinUpdateStatus = ps_json(
        r#"$last = try { (New-Object -ComObject Microsoft.Update.AutoUpdate).Results.LastSearchSuccessDate.ToString('yyyy-MM-dd') } catch { '' };
        [pscustomobject]@{ RebootRequired = (New-Object -ComObject Microsoft.Update.SystemInfo).RebootRequired; LastCheck = "$last" } | ConvertTo-Json -Compress"#,
    )?;
    st.updates = ps_json::<OneOrMany<WinUpdate>>(
        r#"$r = (New-Object -ComObject Microsoft.Update.Session).CreateUpdateSearcher().Search("IsInstalled=0 and IsHidden=0");
        @($r.Updates | ForEach-Object { [pscustomobject]@{ Title = $_.Title; Kind = if ($_.Type -eq 2) { 'Driver' } else { 'Software' };
          Size = [int64]$_.MaxDownloadSize; Mandatory = [bool]$_.IsMandatory } }) | ConvertTo-Json -Compress"#,
    )
    .map(Vec::from)
    .unwrap_or_default();
    Ok(st)
}

/// Ouvre Paramètres → Windows Update : c'est Windows qui installe.
pub fn open_windows_update() -> Result<(), String> {
    open_uri("ms-settings:windowsupdate")
}

/// Ouvre un écran de Windows (`ms-settings:…`). L'appelant vérifie que l'adresse est dans une liste fermée.
pub fn open_uri(uri: &str) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let w = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let (verb, target) = (w("open"), w(uri));
    let r = unsafe { ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), target.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL) };
    if r as isize > 32 { Ok(()) } else { Err(format!("Could not open {uri}.")) }
}

// ---------- Applications (winget) ----------

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct AppUpdate {
    pub name: String,
    pub id: String,
    pub version: String,
    pub available: String,
}

/// Lit le tableau de `winget upgrade` (colonnes à largeur fixe, en-têtes traduits selon la langue).
/// Les positions des colonnes viennent de la ligne d'en-tête, juste au-dessus de la ligne de tirets.
pub fn parse_winget_table(text: &str) -> Vec<AppUpdate> {
    // Barre de progression en début de sortie : garder seulement ce qui suit le dernier retour chariot.
    let lines: Vec<String> = text.lines().map(|l| l.rsplit('\r').next().unwrap_or(l).to_string()).collect();
    let Some(dash) = lines.iter().position(|l| l.trim_start().starts_with("---")) else { return Vec::new() };
    let Some(header) = dash.checked_sub(1).map(|i| &lines[i]) else { return Vec::new() };
    let hc: Vec<char> = header.chars().collect();
    // Début de chaque colonne : un caractère non blanc précédé d'un blanc (ou en tout début).
    let starts: Vec<usize> = (0..hc.len()).filter(|&i| !hc[i].is_whitespace() && (i == 0 || hc[i - 1].is_whitespace())).collect();
    if starts.len() < 4 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for line in &lines[dash + 1..] {
        let c: Vec<char> = line.chars().collect();
        // Fin du tableau : ligne vide ou phrase récapitulative (« 3 mises à niveau disponibles »).
        if c.len() < starts[3] || line.trim().is_empty() {
            break;
        }
        let col = |k: usize| -> String {
            let a = starts[k].min(c.len());
            let b = starts.get(k + 1).copied().unwrap_or(c.len()).min(c.len());
            c[a..b].iter().collect::<String>().trim().to_string()
        };
        let (name, id, version, available) = (col(0), col(1), col(2), col(3));
        if id.is_empty() || available.is_empty() || id.contains(' ') {
            continue;
        }
        out.push(AppUpdate { name, id, version, available });
    }
    out
}

fn winget(args: &[&str]) -> Result<String, String> {
    let out = Command::new("winget")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|_| "winget is not installed (App Installer from the Microsoft Store).".to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Applications qui ont une version plus récente (via winget). Lecture seule.
pub fn app_updates() -> Result<Vec<AppUpdate>, String> {
    winget(&["upgrade", "--include-unknown", "--accept-source-agreements", "--disable-interactivity"]).map(|t| parse_winget_table(&t))
}

/// Met à jour les applications choisies avec winget. Les identifiants viennent de la dernière liste.
pub fn upgrade_apps(ids: &[String], journal_dir: &Path) -> Result<(u64, String), String> {
    let mut ok = 0;
    let mut log = String::new();
    for id in ids {
        let out = Command::new("winget")
            .args([
                "upgrade",
                "--id",
                id,
                "--exact",
                "--silent",
                "--accept-package-agreements",
                "--accept-source-agreements",
                "--disable-interactivity",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            ok += 1;
        }
        log.push_str(&format!("> winget upgrade --id {id}\n{}\n", String::from_utf8_lossy(&out.stdout).lines().last().unwrap_or("").trim()));
    }
    let mut j = journal::Entry::new("app-upgrade");
    j.paths = ids.to_vec();
    j.files = ok;
    j.skipped = ids.len() as u64 - ok;
    j.output = Some(log.clone());
    journal::append(journal_dir, &j).map_err(|e| format!("Log: {e}"))?;
    Ok((ok, log))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winget_table_english() {
        let t = "   - \r   \\ \rName                  Id                     Version   Available Source\n\
                 -----------------------------------------------------------------------\n\
                 7-Zip 24.08 (x64)     7zip.7zip              24.08     25.01     winget\n\
                 Microsoft Edge        Microsoft.Edge         140.0.1   141.0.2   winget\n\
                 2 upgrades available.\n";
        let apps = parse_winget_table(t);
        assert_eq!(apps.len(), 2, "{apps:?}");
        assert_eq!(apps[0], AppUpdate { name: "7-Zip 24.08 (x64)".into(), id: "7zip.7zip".into(), version: "24.08".into(), available: "25.01".into() });
        assert_eq!(apps[1].id, "Microsoft.Edge");
    }

    #[test]
    fn winget_table_french_and_empty() {
        let t = "Nom          ID             Version Disponible Source\n----------------------------------------------\nGit          Git.Git        2.45.0  2.47.1     winget\n";
        assert_eq!(parse_winget_table(t)[0].available, "2.47.1");
        assert!(parse_winget_table("Aucune mise à niveau disponible.\n").is_empty());
    }

    #[test]
    fn support_dates() {
        assert_eq!(support_end(19045), Some("2025-10-14"));
        assert_eq!(support_end(26200), Some("2027-10-12"));
        assert_eq!(support_end(99999), None);
    }

    /// Sur la vraie machine : `cargo test --release system::tests::real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_machine() {
        let i = info().unwrap();
        println!("{} {} build {} · fin {:?} · {} {} · BIOS {} {}", i.os_name, i.os_release, i.os_build, i.support_end, i.manufacturer, i.model, i.bios_version, i.bios_date);
        for g in &i.gpus {
            println!("GPU {} {} {}", g.name, g.driver_version, g.driver_date);
        }
        for d in &i.disks {
            println!("disque {} {} {}", d.name, d.media, d.health);
        }
        let apps = app_updates().unwrap();
        println!("{} applis à mettre à jour", apps.len());
        for a in apps.iter().take(5) {
            println!("  {} {} → {}", a.id, a.version, a.available);
        }
    }
}
