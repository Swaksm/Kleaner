//! Onglet « Mises à jour et santé » : Windows Update, applications (winget), pilotes graphiques, état de la machine.
//! Kleaner ne télécharge rien lui-même : il lit l'état via les outils de Windows, et les mises à jour passent
//! par Windows Update ou winget, seulement quand l'utilisateur clique.

use crate::journal;
use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

/// Lance un programme sans ouvrir de fenêtre de console.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Texte entre apostrophes pour un script PowerShell. PowerShell ferme aussi la chaîne sur ‘ ’ ‚ ‛ :
/// elles sont doublées comme ' (un nom d'utilisateur « D’Artagnan » ou un nom de tâche ne sort jamais de la chaîne).
pub(crate) fn ps_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') {
            out.push(c);
        }
        out.push(c);
    }
    out.push('\'');
    out
}

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
    pub ltsc: bool,
    #[serde(skip_deserializing)]
    pub gpus: Vec<Gpu>,
    #[serde(skip_deserializing)]
    pub disks: Vec<Disk>,
}

fn current_version(name: &str) -> Option<String> {
    winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .and_then(|k| k.get_value::<String, _>(name))
        .ok()
}

/// Numéro de build de Windows (22000 et plus : Windows 11), 0 si illisible.
pub fn windows_build() -> u32 {
    current_version("CurrentBuildNumber").and_then(|b| b.parse().ok()).unwrap_or(0)
}

/// Édition de Windows (`EditionID` : Core, Professional, Enterprise, EnterpriseS…), vide si illisible.
pub fn windows_edition() -> String {
    current_version("EditionID").unwrap_or_default()
}

/// Édition LTSC (Enterprise LTSC, IoT Enterprise LTSC) : Windows Update n'y installe jamais de nouvelle version.
pub fn is_ltsc(edition: &str) -> bool {
    matches!(edition.strip_suffix('N').unwrap_or(edition), "EnterpriseS" | "IoTEnterpriseS")
}

/// Fin du support par build et édition, dates de learn.microsoft.com/lifecycle (dernier Patch Tuesday).
/// Édition ou build absents de la table : `None`, Kleaner ne devine pas.
// shortcut: table à compléter à chaque nouvelle version de Windows (les builds inconnues n'affichent rien).
pub fn support_end(build: u32, edition: &str) -> Option<&'static str> {
    // Les éditions « N » (sans lecteur multimédia) ont le même cycle de vie.
    let e = edition.strip_suffix('N').unwrap_or(edition);
    let home_pro = matches!(
        e,
        "Core" | "CoreSingleLanguage" | "CoreCountrySpecific" | "Professional" | "ProfessionalEducation" | "ProfessionalWorkstation" | "CloudEdition"
    );
    let enterprise = matches!(e, "Enterprise" | "Education" | "ServerRdsh" | "IoTEnterprise");
    Some(match (e, build) {
        // LTSC : cycle fixe de 5 ans (10 ans pour IoT).
        ("IoTEnterpriseS", 26100) => "2034-10-10", // Windows 11 IoT Enterprise LTSC 2024
        ("EnterpriseS", 26100) => "2029-10-09",    // Windows 11 Enterprise LTSC 2024
        ("IoTEnterpriseS", 19044) => "2032-01-13", // Windows 10 IoT Enterprise LTSC 2021
        ("EnterpriseS", 19044) => "2027-01-12",    // Windows 10 Enterprise LTSC 2021
        ("IoTEnterpriseS" | "EnterpriseS", 17763) => "2029-01-09", // LTSC 2019
        ("EnterpriseS", 14393) => "2026-10-13",    // LTSB 2016
        // Windows 10 : fin pour toutes les versions classiques le 14 octobre 2025.
        (_, 10240..=19045) if home_pro || enterprise => "2025-10-14",
        // Windows 11 Famille et Pro : 24 mois par version.
        (_, 22000) if home_pro => "2023-10-10", // 21H2
        (_, 22621) if home_pro => "2024-10-08", // 22H2
        (_, 22631) if home_pro => "2025-11-11", // 23H2
        (_, 26100) if home_pro => "2026-10-13", // 24H2
        (_, 26200) if home_pro => "2027-10-12", // 25H2
        (_, 28000) if home_pro => "2028-03-14", // 26H1
        (_, 26300) if home_pro => "2028-10-09", // 26H2
        // Windows 11 Entreprise et Éducation : 36 mois pour les versions de l'automne.
        (_, 22000) if enterprise => "2024-10-08",
        (_, 22621) if enterprise => "2025-10-14",
        (_, 22631) if enterprise => "2026-11-10",
        (_, 26100) if enterprise => "2027-10-12",
        (_, 26200) if enterprise => "2028-10-10",
        (_, 28000) if enterprise => "2029-03-13",
        (_, 26300) if enterprise => "2029-10-09",
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
    let edition = windows_edition();
    s.support_end = support_end(s.os_build, &edition).map(String::from);
    s.ltsc = is_ltsc(&edition);
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
    if shell_execute("open", uri.as_ref(), "") { Ok(()) } else { Err(format!("Could not open {uri}.")) }
}

/// ShellExecuteW (fichier, adresse ou programme, avec l'UAC si `verb` vaut « runas » ou si le programme l'exige).
pub(crate) fn shell_execute(verb: &str, file: &std::ffi::OsStr, args: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let w = |s: &std::ffi::OsStr| s.encode_wide().chain([0]).collect::<Vec<u16>>();
    let (verb, file, args) = (w(verb.as_ref()), w(file), w(args.as_ref()));
    let params = if args.len() > 1 { args.as_ptr() } else { std::ptr::null() };
    let r = unsafe { ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(), params, std::ptr::null(), SW_SHOWNORMAL) };
    // ShellExecute renvoie une valeur > 32 en cas de succès.
    r as isize > 32
}

// ---------- Applications (winget) ----------

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct AppUpdate {
    pub name: String,
    pub id: String,
    pub version: String,
    pub available: String,
    /// L'appli se met à jour toute seule : winget n'est pas proposé (voir `SELF_UPDATING`).
    pub self_update: bool,
}

/// Lanceurs qui se mettent à jour eux-mêmes à l'ouverture sans changer la version lue par winget :
/// winget croit qu'une mise à jour existe pour toujours, et son installeur échoue ou ne change rien.
/// Exemple : Epic Games Launcher reste en 1.3.150.0 dans la liste des programmes alors que l'exe est à jour.
pub const SELF_UPDATING: &[&str] =
    &["EpicGames.EpicGamesLauncher", "Valve.Steam", "Blizzard.BattleNet", "ElectronicArts.EADesktop", "Ubisoft.Connect", "GOG.Galaxy"];

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
        // shortcut: identifiant coupé par winget (« … ») ignoré ; lire `winget export` si des applis manquent.
        if id.is_empty() || available.is_empty() || id.contains(' ') || id.ends_with('…') {
            continue;
        }
        let self_update = SELF_UPDATING.contains(&id.as_str());
        out.push(AppUpdate { name, id, version, available, self_update });
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
/// Sans `--include-unknown` : une appli à version inconnue n'a pas de mise à jour certaine.
pub fn app_updates() -> Result<Vec<AppUpdate>, String> {
    winget(&["upgrade", "--accept-source-agreements", "--disable-interactivity"]).map(|t| parse_winget_table(&t))
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
        // Les dernières lignes de winget disent pourquoi (appli ouverte, installateur interactif, hash…).
        let text = String::from_utf8_lossy(&out.stdout);
        let tail: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with(['-', '\\', '|', '/'])).collect();
        let tail = tail[tail.len().saturating_sub(3)..].join("\n");
        log.push_str(&format!("> winget upgrade --id {id} (exit {})\n{tail}\n\n", out.status.code().unwrap_or(-1)));
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

    /// Le vrai PowerShell relit exactement le texte, même avec des apostrophes typographiques.
    #[test]
    fn ps_quote_survives_every_apostrophe() {
        let tricky = "D’Artagnan's ‘task’ ‚x‛; Write-Output pwned; '";
        let (out, err, ok) = ps_run(&format!("Write-Output {}", ps_quote(tricky))).unwrap();
        assert!(ok, "{err}");
        assert_eq!(out, tricky);
    }

    #[test]
    fn winget_table_english() {
        let t = "   - \r   \\ \rName                  Id                     Version   Available Source\n\
                 -----------------------------------------------------------------------\n\
                 7-Zip 24.08 (x64)     7zip.7zip              24.08     25.01     winget\n\
                 Microsoft Edge        Microsoft.Edge         140.0.1   141.0.2   winget\n\
                 2 upgrades available.\n";
        let apps = parse_winget_table(t);
        assert_eq!(apps.len(), 2, "{apps:?}");
        assert_eq!(apps[0], AppUpdate { name: "7-Zip 24.08 (x64)".into(), id: "7zip.7zip".into(), version: "24.08".into(), available: "25.01".into(), self_update: false });
        assert_eq!(apps[1].id, "Microsoft.Edge");
    }

    #[test]
    fn winget_table_french_and_empty() {
        let t = "Nom          ID             Version Disponible Source\n----------------------------------------------\nGit          Git.Git        2.45.0  2.47.1     winget\n";
        assert_eq!(parse_winget_table(t)[0].available, "2.47.1");
        assert!(parse_winget_table("Aucune mise à niveau disponible.\n").is_empty());
    }

    #[test]
    fn winget_truncated_id_is_skipped() {
        let t = "Name        Id                 Version Available Source\n-------------------------------------------------------\nVS Code     Microsoft.VisualS… 1.0     1.1       winget\nGit         Git.Git            2.45.0  2.47.1    winget\n";
        let apps = parse_winget_table(t);
        assert_eq!(apps.len(), 1, "{apps:?}");
        assert_eq!(apps[0].id, "Git.Git");
        assert!(!apps[0].self_update);
    }

    #[test]
    fn winget_self_updating_launcher() {
        let t = "Name                 Id                          Version   Available Source\n---------------------------------------------------------------------------\nEpic Games Launcher  EpicGames.EpicGamesLauncher 1.3.150.0 1.3.210.0 winget\n";
        assert!(parse_winget_table(t)[0].self_update);
    }

    #[test]
    fn support_dates() {
        assert_eq!(support_end(19045, "Professional"), Some("2025-10-14"));
        assert_eq!(support_end(26200, "Core"), Some("2027-10-12"));
        assert_eq!(support_end(26100, "ProfessionalN"), Some("2026-10-13"));
        // Même build 26100 : la LTSC n'a pas la date de Famille et Pro.
        assert_eq!(support_end(26100, "IoTEnterpriseS"), Some("2034-10-10"));
        assert_eq!(support_end(26100, "EnterpriseS"), Some("2029-10-09"));
        assert_eq!(support_end(26100, "Enterprise"), Some("2027-10-12"));
        assert_eq!(support_end(19044, "EnterpriseSN"), Some("2027-01-12"));
        assert_eq!(support_end(26300, "Education"), Some("2029-10-09"));
        assert_eq!(support_end(99999, "Professional"), None);
        assert_eq!(support_end(26100, "EnterpriseG"), None);
        assert_eq!(support_end(26100, ""), None);
        assert!(is_ltsc("IoTEnterpriseS") && is_ltsc("EnterpriseSN") && !is_ltsc("Enterprise"));
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
