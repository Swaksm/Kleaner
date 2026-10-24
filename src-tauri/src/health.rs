//! Bilan de santé et de sécurité : lecture seule. Kleaner ne change aucun réglage de Windows : chaque alerte
//! renvoie vers l'écran Windows qui la règle. Ce n'est pas un antivirus.

use crate::catalog::Text;
use crate::scan::{self, DriveKind};
use crate::system::{self, ps_json};
use serde::{Deserialize, Serialize};
use winreg::RegKey;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

/// Pages Windows que Kleaner sait ouvrir (liste fermée : l'interface n'envoie jamais d'adresse libre).
pub const PAGES: [&str; 8] = [
    "ms-settings:windowsupdate",
    "ms-settings:windowsdefender",
    "windowsdefender://network",
    "ms-settings:deviceencryption",
    "ms-settings:powersleep",
    "ms-settings:storagesense",
    "ms-settings:recovery",
    "windowsdefender://devicesecurity",
];

#[derive(Serialize, Clone, Debug)]
pub struct HealthItem {
    pub id: &'static str,
    /// system, disk ou security.
    pub group: &'static str,
    /// ok, warn, bad, info ou unknown.
    pub status: &'static str,
    pub title: Text,
    pub detail: Text,
    /// Page Windows qui règle le problème (une entrée de `PAGES`).
    pub fix: Option<&'static str>,
}

#[derive(Serialize, Clone, Debug)]
pub struct HealthReport {
    pub items: Vec<HealthItem>,
    pub elevated: bool,
}

fn t(en: impl Into<String>, fr: impl Into<String>) -> Text {
    Text::new(en, fr)
}

fn item(id: &'static str, group: &'static str, status: &'static str, title: Text, detail: Text, fix: Option<&'static str>) -> HealthItem {
    HealthItem { id, group, status, title, detail, fix }
}

fn hklm_key(path: &str) -> Option<RegKey> {
    RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(path, KEY_READ).ok()
}

fn hklm_dword(path: &str, name: &str) -> Option<u32> {
    hklm_key(path)?.get_value::<u32, _>(name).ok()
}

fn hklm_string(path: &str, name: &str) -> Option<String> {
    hklm_key(path)?.get_value::<String, _>(name).ok()
}

// ---------- Jours (sans dépendance) ----------

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// « 2027-10-12 » → jours depuis 1970.
fn iso_days(s: &str) -> Option<i64> {
    let mut p = s.split('-').map(|x| x.parse::<i64>().ok());
    Some(days_from_civil(p.next()??, p.next()??, p.next()??))
}

fn today_days() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| (d.as_secs() / 86_400) as i64)
}

// ---------- Disques ----------

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct DiskRow {
    name: String,
    media: String,
    health: String,
    wear: i64,
    temp: i64,
    hours: i64,
}

fn disks() -> Vec<HealthItem> {
    let rows: Vec<DiskRow> = ps_json(
        r#"ConvertTo-Json -Compress -InputObject @(Get-PhysicalDisk -ErrorAction SilentlyContinue | ForEach-Object {
          $c = $_ | Get-StorageReliabilityCounter -ErrorAction SilentlyContinue;
          [pscustomobject]@{ Name = $_.FriendlyName; Media = "$($_.MediaType)"; Health = "$($_.HealthStatus)";
            Wear = if ($c -and $null -ne $c.Wear) { [int]$c.Wear } else { -1 };
            Temp = if ($c -and $null -ne $c.Temperature) { [int]$c.Temperature } else { -1 };
            Hours = if ($c -and $null -ne $c.PowerOnHours) { [int64]$c.PowerOnHours } else { -1 } } })"#,
    )
    .unwrap_or_default();
    rows.into_iter()
        .map(|d| {
            let mut en = Vec::new();
            let mut fr = Vec::new();
            if d.wear >= 0 {
                en.push(format!("wear {}%", d.wear));
                fr.push(format!("usure {} %", d.wear));
            }
            if d.temp > 0 {
                en.push(format!("{} °C", d.temp));
                fr.push(format!("{} °C", d.temp));
            }
            if d.hours >= 0 {
                en.push(format!("{} h powered on", d.hours));
                fr.push(format!("{} h de fonctionnement", d.hours));
            }
            let known = !en.is_empty();
            let status = if d.health != "Healthy" && !d.health.is_empty() || d.wear >= 90 || d.temp >= 75 {
                "bad"
            } else if d.wear >= 70 || d.temp >= 60 {
                "warn"
            } else if d.health.is_empty() {
                "unknown"
            } else {
                "ok"
            };
            let more = if known { "" } else { " Run Kleaner as administrator to read wear and temperature." };
            let plus = if known { "" } else { " Lance Kleaner en administrateur pour lire l’usure et la température." };
            let health = if d.health.is_empty() { "unknown".to_string() } else { d.health.clone() };
            item(
                "disk",
                "disk",
                status,
                t("Disk {name} ({media})", "Disque {name} ({media})").with(&[("name", &d.name), ("media", &d.media)]),
                t(
                    format!("Health: {health}{}{}.{more}", if known { ", " } else { "" }, en.join(", ")),
                    format!("État : {health}{}{}.{plus}", if known { ", " } else { "" }, fr.join(", ")),
                ),
                None,
            )
        })
        .collect()
}

fn space() -> Vec<HealthItem> {
    scan::drives()
        .into_iter()
        .filter(|d| d.kind == DriveKind::Fixed && d.total > 0)
        .map(|d| {
            let pct = d.free * 100 / d.total;
            let status = if pct < 5 { "bad" } else if pct < 10 { "warn" } else { "ok" };
            let gb = d.free as f64 / 1e9;
            item(
                "space",
                "disk",
                status,
                t("Free space on {root}", "Espace libre sur {root}").with(&[("root", &d.root)]),
                t(
                    "{gb} GB free ({pct}%). Windows slows down and cannot update below about 10%.",
                    "{gb} Go libres ({pct} %). Sous 10 % environ, Windows ralentit et ne peut plus se mettre à jour.",
                )
                .with(&[("gb", &format!("{gb:.1}")), ("pct", &pct.to_string())]),
                (status != "ok").then_some("ms-settings:storagesense"),
            )
        })
        .collect()
}

// ---------- Batterie, plantages ----------

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct Battery {
    full: i64,
    design: i64,
}

fn battery() -> Option<HealthItem> {
    let b: Battery = ps_json(
        r#"$f = Get-CimInstance -Namespace root\wmi -ClassName BatteryFullChargedCapacity -ErrorAction SilentlyContinue | Select-Object -First 1;
        $s = Get-CimInstance -Namespace root\wmi -ClassName BatteryStaticData -ErrorAction SilentlyContinue | Select-Object -First 1;
        ConvertTo-Json -Compress -InputObject ([pscustomobject]@{ Full = [int64]$f.FullChargedCapacity; Design = [int64]$s.DesignedCapacity })"#,
    )
    .ok()?;
    if b.design <= 0 || b.full <= 0 {
        return None;
    }
    let pct = (b.full * 100 / b.design).min(100);
    let status = if pct < 40 { "bad" } else if pct < 60 { "warn" } else { "ok" };
    Some(item(
        "battery",
        "system",
        status,
        t("Battery", "Batterie"),
        t(
            "It now holds {pct}% of its original capacity. Below 60% a replacement is worth considering.",
            "Elle ne tient plus que {pct} % de sa capacité d’origine. Sous 60 %, un remplacement se justifie.",
        )
        .with(&[("pct", &pct.to_string())]),
        None,
    ))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct CrashApp {
    name: String,
    count: u64,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct Crashes {
    apps: Vec<CrashApp>,
    bsod: u64,
}

fn crashes() -> Vec<HealthItem> {
    let c: Crashes = ps_json(
        r#"$since = (Get-Date).AddDays(-30);
        $apps = @(Get-WinEvent -FilterHashtable @{ LogName = 'Application'; Id = 1000; StartTime = $since } -MaxEvents 1000 -ErrorAction SilentlyContinue |
          ForEach-Object { $_.Properties[0].Value } | Group-Object | Sort-Object Count -Descending | Select-Object -First 5 |
          ForEach-Object { [pscustomobject]@{ Name = $_.Name; Count = $_.Count } });
        $bsod = @(Get-WinEvent -FilterHashtable @{ LogName = 'System'; ProviderName = 'Microsoft-Windows-WER-SystemErrorReporting'; Id = 1001; StartTime = $since } -ErrorAction SilentlyContinue).Count;
        ConvertTo-Json -Compress -Depth 4 -InputObject ([pscustomobject]@{ Apps = $apps; Bsod = [int]$bsod })"#,
    )
    .unwrap_or_default();
    let mut out = Vec::new();
    out.push(item(
        "bsod",
        "system",
        match c.bsod {
            0 => "ok",
            1..=2 => "warn",
            _ => "bad",
        },
        t("Blue screens (30 days)", "Écrans bleus (30 jours)"),
        if c.bsod == 0 {
            t("None in the last 30 days.", "Aucun ces 30 derniers jours.")
        } else {
            t(
                "{count} in the last 30 days. Update your drivers (Updates tab) and check the disk and memory.",
                "{count} ces 30 derniers jours. Mets à jour les pilotes (onglet Mises à jour), vérifie le disque et la mémoire.",
            )
            .with(&[("count", &c.bsod.to_string())])
        },
        None,
    ));
    let top = c.apps.first();
    let list = c.apps.iter().map(|a| format!("{} ×{}", a.name, a.count)).collect::<Vec<_>>().join(", ");
    out.push(item(
        "app-crashes",
        "system",
        if top.is_some_and(|a| a.count >= 10) { "warn" } else { "ok" },
        t("Application crashes (30 days)", "Plantages d’applications (30 jours)"),
        if c.apps.is_empty() {
            t("No application crash recorded.", "Aucun plantage d’application enregistré.")
        } else {
            t(
                "Most frequent: {list}. A program that crashes often should be updated or reinstalled.",
                "Les plus fréquents : {list}. Un programme qui plante souvent est à mettre à jour ou à réinstaller.",
            )
            .with(&[("list", &list)])
        },
        None,
    ));
    out
}

// ---------- Système ----------

fn reboot_pending() -> HealthItem {
    let pending = ["SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Component Based Servicing\\RebootPending", "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WindowsUpdate\\Auto Update\\RebootRequired"]
        .iter()
        .any(|p| RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(p, KEY_READ).is_ok());
    item(
        "reboot",
        "system",
        if pending { "warn" } else { "ok" },
        t("Restart pending", "Redémarrage en attente"),
        if pending {
            t("Windows is waiting for a restart to finish installing updates.", "Windows attend un redémarrage pour terminer l’installation de mises à jour.")
        } else {
            t("Nothing is waiting for a restart.", "Rien n’attend un redémarrage.")
        },
        pending.then_some("ms-settings:windowsupdate"),
    )
}

fn uptime_and_memory() -> Vec<HealthItem> {
    use windows_sys::Win32::System::SystemInformation::{GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let days = unsafe { GetTickCount64() } / 86_400_000;
    let mut m: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    let ok = unsafe { GlobalMemoryStatusEx(&mut m) } != 0;
    let mut out = vec![item(
        "uptime",
        "system",
        if days >= 30 { "warn" } else { "ok" },
        t("Time since last restart", "Durée depuis le dernier redémarrage"),
        t(
            "{days} days. A restart finishes pending updates and frees leaked memory.",
            "{days} jours. Un redémarrage termine les mises à jour en attente et libère la mémoire.",
        )
        .with(&[("days", &days.to_string())]),
        None,
    )];
    if ok {
        let load = m.dwMemoryLoad;
        out.push(item(
            "memory",
            "system",
            if load >= 90 { "warn" } else { "ok" },
            t("Memory in use", "Mémoire utilisée"),
            t(
                "{load}% of {total} GB right now. Close heavy programs if it stays above 90%.",
                "{load} % de {total} Go en ce moment. Ferme les programmes lourds si ça reste au-dessus de 90 %.",
            )
            .with(&[("load", &load.to_string()), ("total", &format!("{:.0}", m.ullTotalPhys as f64 / 1e9))]),
            None,
        ));
    }
    out
}

fn power_plan() -> Option<HealthItem> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powercfg").arg("/getactivescheme").creation_flags(crate::system::CREATE_NO_WINDOW).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    let (name_en, name_fr, saver) = if text.contains("a1841308-3541-4fab-bc81-f71556f20b4a") {
        ("Power saver", "Économie d’énergie", true)
    } else if text.contains("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c") {
        ("High performance", "Performances élevées", false)
    } else if text.contains("381b4222-f694-41f0-9685-ff5bb260df2e") {
        ("Balanced", "Utilisation normale", false)
    } else {
        return None;
    };
    Some(item(
        "power",
        "system",
        if saver { "info" } else { "ok" },
        t("Power plan", "Mode d’alimentation"),
        if saver {
            t(format!("{name_en}: the processor is slowed down. Choose Balanced if the PC feels slow."), format!("{name_fr} : le processeur est bridé. Choisis « Utilisation normale » si le PC paraît lent."))
        } else {
            t(format!("{name_en}."), format!("{name_fr}."))
        },
        saver.then_some("ms-settings:powersleep"),
    ))
}

fn support() -> Option<HealthItem> {
    let build: u32 = hklm_string(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "CurrentBuildNumber")?.parse().ok()?;
    let end = system::support_end(build)?;
    let left = iso_days(end)? - today_days();
    let status = if left < 0 { "bad" } else if left < 180 { "warn" } else { "ok" };
    Some(item(
        "support",
        "security",
        status,
        t("Windows support", "Support de Windows"),
        if left < 0 {
            t("Security updates stopped on {end}. Move to a supported version of Windows.", "Les mises à jour de sécurité se sont arrêtées le {end}. Passe à une version de Windows encore prise en charge.")
                .with(&[("end", end)])
        } else {
            t("Security updates until {end} ({left} days).", "Mises à jour de sécurité jusqu’au {end} ({left} jours).")
                .with(&[("end", end), ("left", &left.to_string())])
        },
        (status != "ok").then_some("ms-settings:windowsupdate"),
    ))
}

fn last_update() -> Option<HealthItem> {
    let secs: Option<i64> = ps_json(
        r#"$h = Get-HotFix -ErrorAction SilentlyContinue | Where-Object { $_.InstalledOn } | Sort-Object InstalledOn -Descending | Select-Object -First 1;
        if ($h) { [DateTimeOffset]::new($h.InstalledOn).ToUnixTimeSeconds() } else { 'null' }"#,
    )
    .ok()?;
    let days = today_days() - secs? / 86_400;
    Some(item(
        "last-update",
        "security",
        if days > 90 { "warn" } else { "ok" },
        t("Latest Windows update installed", "Dernière mise à jour Windows installée"),
        t("{days} days ago. Security updates arrive about every month.", "Il y a {days} jours. Les mises à jour de sécurité arrivent environ chaque mois.")
            .with(&[("days", &days.to_string())]),
        (days > 90).then_some("ms-settings:windowsupdate"),
    ))
}

// ---------- Sécurité ----------

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct Av {
    name: String,
    on: bool,
    current: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct Protection {
    av: Vec<Av>,
    /// Pare-feu : « Domain=True,Private=True,Public=False ».
    firewall: Vec<String>,
    /// 1 actif, 2 inactif, 3 en cours, 5 suspendu… ; -1 inconnu.
    bitlocker: i64,
    tpm: String,
}

fn protection() -> Vec<HealthItem> {
    let p: Protection = ps_json(
        r#"$av = @(Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntivirusProduct -ErrorAction SilentlyContinue | ForEach-Object {
          [pscustomobject]@{ Name = $_.displayName; On = ((($_.productState -shr 12) -band 0xF) -eq 1); Current = ((($_.productState -shr 4) -band 0xF) -eq 0) } });
        $fw = @(Get-NetFirewallProfile -ErrorAction SilentlyContinue | ForEach-Object { "$($_.Name)=$($_.Enabled)" });
        $bl = try { [int](New-Object -ComObject Shell.Application).NameSpace("$env:SystemDrive\").Self.ExtendedProperty('System.Volume.BitLockerProtection') } catch { -1 };
        $tpm = try { $x = Get-CimInstance -Namespace 'root\cimv2\Security\MicrosoftTpm' -ClassName Win32_Tpm -ErrorAction Stop; if ($x -and $x.IsEnabled_InitialValue) { "on $($x.SpecVersion)" } else { 'off' } } catch { 'unknown' };
        ConvertTo-Json -Compress -Depth 4 -InputObject ([pscustomobject]@{ Av = $av; Firewall = $fw; Bitlocker = $bl; Tpm = $tpm })"#,
    )
    .unwrap_or(Protection { bitlocker: -1, ..Default::default() });
    let mut out = Vec::new();

    let names = p.av.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", ");
    let active: Vec<&Av> = p.av.iter().filter(|a| a.on).collect();
    let (status, detail) = if p.av.is_empty() {
        ("unknown", t("Windows did not report any antivirus.", "Windows n’a signalé aucun antivirus."))
    } else if active.is_empty() {
        ("bad", t("{names}: protection is turned off.", "{names} : la protection est désactivée.").with(&[("names", &names)]))
    } else if active.iter().any(|a| !a.current) {
        ("warn", t("{names}: virus definitions are out of date.", "{names} : les définitions de virus ne sont pas à jour.").with(&[("names", &names)]))
    } else {
        ("ok", t("{names}: active and up to date.", "{names} : actif et à jour.").with(&[("names", &names)]))
    };
    out.push(item("antivirus", "security", status, t("Antivirus", "Antivirus"), detail, (status == "bad" || status == "warn").then_some("ms-settings:windowsdefender")));

    if !p.firewall.is_empty() {
        let off: Vec<&str> = p.firewall.iter().filter(|f| f.ends_with("=False")).map(|f| f.split('=').next().unwrap_or("")).collect();
        let status = if off.contains(&"Public") { "bad" } else if off.is_empty() { "ok" } else { "warn" };
        out.push(item(
            "firewall",
            "security",
            status,
            t("Firewall", "Pare-feu"),
            if off.is_empty() {
                t("On for every network profile.", "Activé pour tous les types de réseau.")
            } else {
                t("Turned off for: {list}.", "Désactivé pour : {list}.").with(&[("list", &off.join(", "))])
            },
            (status != "ok").then_some("windowsdefender://network"),
        ));
    }

    let uac_key = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System";
    if let Some(lua) = hklm_dword(uac_key, "EnableLUA") {
        let silent = hklm_dword(uac_key, "ConsentPromptBehaviorAdmin") == Some(0);
        let status = if lua == 0 { "bad" } else if silent { "warn" } else { "ok" };
        out.push(item(
            "uac",
            "security",
            status,
            t("User Account Control (UAC)", "Contrôle de compte d’utilisateur (UAC)"),
            match status {
                "bad" => t("Turned off: any program can change the system without asking. Search « Change User Account Control settings » in the Start menu.", "Désactivé : n’importe quel programme peut modifier le système sans demander. Cherche « Modifier les paramètres de contrôle de compte d’utilisateur » dans le menu Démarrer."),
                "warn" => t("Set to never notify. Search « Change User Account Control settings » in the Start menu.", "Réglé sur ne jamais avertir. Cherche « Modifier les paramètres de contrôle de compte d’utilisateur » dans le menu Démarrer."),
                _ => t("On: Windows asks before a program changes the system.", "Activé : Windows demande avant qu’un programme modifie le système."),
            },
            None,
        ));
    }

    if let Some(s) = hklm_string(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer", "SmartScreenEnabled") {
        let off = s.eq_ignore_ascii_case("off");
        out.push(item(
            "smartscreen",
            "security",
            if off { "warn" } else { "ok" },
            t("SmartScreen", "SmartScreen"),
            if off {
                t("Off: downloaded programs are not checked against known threats.", "Désactivé : les programmes téléchargés ne sont pas comparés aux menaces connues.")
            } else {
                t("On: unknown downloaded programs trigger a warning.", "Activé : un programme téléchargé inconnu déclenche un avertissement.")
            },
            off.then_some("ms-settings:windowsdefender"),
        ));
    }

    let sb = hklm_dword(r"SYSTEM\CurrentControlSet\Control\SecureBoot\State", "UEFISecureBootEnabled");
    out.push(item(
        "secureboot",
        "security",
        match sb {
            Some(1) => "ok",
            Some(_) => "warn",
            None => "info",
        },
        t("Secure Boot", "Démarrage sécurisé (Secure Boot)"),
        match sb {
            Some(1) => t("On: only trusted software starts before Windows.", "Activé : seul un logiciel de confiance démarre avant Windows."),
            Some(_) => t("Off. It is turned on in the PC firmware (UEFI) settings.", "Désactivé. Il s’active dans les réglages du micrologiciel (UEFI) du PC."),
            None => t("Not available: this PC starts in legacy BIOS mode.", "Non disponible : ce PC démarre en mode BIOS classique."),
        },
        (sb == Some(0)).then_some("ms-settings:recovery"),
    ));

    out.push(item(
        "tpm",
        "security",
        if p.tpm.starts_with("on") { "ok" } else if p.tpm == "off" { "warn" } else { "unknown" },
        t("TPM security chip", "Puce de sécurité TPM"),
        if let Some(v) = p.tpm.strip_prefix("on ") {
            t("Present and enabled (version {v}).", "Présente et activée (version {v}).").with(&[("v", v)])
        } else if p.tpm == "off" {
            t("Missing or turned off in the firmware. Windows 11 requires it.", "Absente ou désactivée dans le micrologiciel. Windows 11 en a besoin.")
        } else {
            t("Reading it needs administrator rights.", "Sa lecture demande les droits administrateur.")
        },
        (p.tpm == "off").then_some("windowsdefender://devicesecurity"),
    ));

    out.push(item(
        "bitlocker",
        "security",
        match p.bitlocker {
            1 => "ok",
            5 => "warn",
            -1 => "unknown",
            _ => "info",
        },
        t("Drive encryption (BitLocker)", "Chiffrement du disque (BitLocker)"),
        match p.bitlocker {
            1 => t("System drive is encrypted: nobody can read it if the PC is stolen.", "Le disque système est chiffré : personne ne peut le lire si le PC est volé."),
            5 => t("Protection is suspended. Resume it in Windows settings.", "La protection est suspendue. Reprends-la dans les paramètres de Windows."),
            -1 => t("Could not be read.", "Lecture impossible."),
            3 | 4 => t("Encryption is in progress.", "Le chiffrement est en cours."),
            _ => t("Off. Worth turning on on a laptop; keep your recovery key safe.", "Désactivé. À activer sur un portable ; garde ta clé de récupération en lieu sûr."),
        },
        matches!(p.bitlocker, 5 | 0 | 2).then_some("ms-settings:deviceencryption"),
    ));
    out
}

pub fn report() -> HealthReport {
    let items = std::thread::scope(|s| {
        let jobs: Vec<std::thread::ScopedJoinHandle<Vec<HealthItem>>> = vec![
            s.spawn(disks),
            s.spawn(|| battery().into_iter().collect()),
            s.spawn(crashes),
            s.spawn(protection),
            s.spawn(|| last_update().into_iter().collect()),
            s.spawn(|| {
                let mut v = vec![reboot_pending()];
                v.extend(uptime_and_memory());
                v.extend(power_plan());
                v.extend(support());
                v.extend(space());
                v
            }),
        ];
        jobs.into_iter().flat_map(|j| j.join().unwrap_or_default()).collect::<Vec<_>>()
    });
    HealthReport { items, elevated: crate::elevation::is_elevated() }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct Dism {
    state: String,
}

/// Contrôle d'intégrité des composants de Windows (lecture seule, droits admin) : `Repair-WindowsImage -CheckHealth`.
pub fn dism_check() -> Result<HealthItem, String> {
    if !crate::elevation::is_elevated() {
        return Err("Administrator rights are needed: relaunch Kleaner as administrator".into());
    }
    let d: Dism = ps_json(
        r#"ConvertTo-Json -Compress -InputObject ([pscustomobject]@{ State = "$((Repair-WindowsImage -Online -CheckHealth -ErrorAction Stop).ImageHealthState)" })"#,
    )?;
    let (status, detail) = match d.state.as_str() {
        "Healthy" => ("ok", t("No corruption found in the Windows component store.", "Aucune corruption trouvée dans le magasin de composants de Windows.")),
        "Repairable" => ("warn", t("Corruption found, and it can be repaired. In an administrator terminal: DISM /Online /Cleanup-Image /RestoreHealth, then sfc /scannow.", "Corruption trouvée, réparable. Dans un terminal administrateur : DISM /Online /Cleanup-Image /RestoreHealth, puis sfc /scannow.")),
        _ => ("bad", t("The component store cannot be repaired. A repair install of Windows may be needed.", "Le magasin de composants n’est pas réparable. Une réinstallation de réparation de Windows peut être nécessaire.")),
    };
    Ok(item("dism", "system", status, t("Windows files integrity (DISM)", "Intégrité des fichiers de Windows (DISM)"), detail, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_counted_in_days() {
        assert_eq!(iso_days("1970-01-01"), Some(0));
        assert_eq!(iso_days("2027-10-12").unwrap() - iso_days("2027-10-11").unwrap(), 1);
        assert_eq!(iso_days("2024-03-01").unwrap() - iso_days("2024-02-28").unwrap(), 2, "année bissextile");
        assert_eq!(iso_days("pas une date"), None);
    }

    #[test]
    fn every_fix_is_in_the_allowed_list() {
        for p in ["ms-settings:windowsupdate", "ms-settings:windowsdefender", "windowsdefender://network", "ms-settings:deviceencryption", "ms-settings:powersleep", "ms-settings:storagesense", "ms-settings:recovery", "windowsdefender://devicesecurity"] {
            assert!(PAGES.contains(&p), "{p}");
        }
    }

    /// Sur la vraie machine : `cargo test --release health::tests::real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real() {
        for i in report().items {
            println!("[{}] {:10} {:5} {} — {}", i.group, i.id, i.status, i.title.en, i.detail.en);
        }
    }
}
