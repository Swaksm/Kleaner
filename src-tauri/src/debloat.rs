//! Applis préinstallées (paquets du Microsoft Store) qu'on peut retirer de son compte. Liste fermée `KNOWN` :
//! un paquet absent de la liste n'est jamais proposé (Kleaner ne devine pas).
//! Retrait : `Remove-AppxPackage` pour l'utilisateur courant seulement, sans admin ; réinstallable depuis le Store.
//! Jamais touchés : paquets provisionnés (futurs comptes), paquets système, frameworks, et tout ce qui est dans `NEVER`
//! (Store, Edge, Photos, Calculatrice, Terminal, Bloc-notes, Paint, Caméra, Capture, extensions, pilotes…).

use crate::journal;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Motif sur le nom du paquet (`Name` de Get-AppxPackage, sans tenir compte de la casse) : exact, `début*`, `*fin` ou `*milieu*`.
struct Known {
    pattern: &'static str,
    name: &'static str,
    /// Clé d'avertissement affichée par l'interface (`debloat.warn.<clé>`).
    warn: Option<&'static str>,
}

const fn k(pattern: &'static str, name: &'static str, warn: Option<&'static str>) -> Known {
    Known { pattern, name, warn }
}

const KNOWN: &[Known] = &[
    // Jeux et applis d'autres éditeurs, souvent installés d'office par Windows ou le fabricant.
    k("king.com.*", "Candy Crush (King)", None),
    k("*.TikTok", "TikTok", None),
    k("Disney.*", "Disney+", None),
    k("SpotifyAB.SpotifyMusic", "Spotify", None),
    k("4DF9E0F8.Netflix", "Netflix", None),
    k("AmazonVideo.PrimeVideo", "Prime Video", None),
    k("Facebook.*", "Facebook / Instagram", None),
    k("*.Twitter", "X (Twitter)", None),
    k("7EE7776C.LinkedInforWindows", "LinkedIn", None),
    k("*.AdobePhotoshopExpress", "Photoshop Express", None),
    k("*.Duolingo*", "Duolingo", None),
    k("*.Flipboard", "Flipboard", None),
    k("*.MarchofEmpires", "March of Empires", None),
    k("*.Asphalt8Airborne", "Asphalt 8", None),
    k("*.HiddenCityMysteryofShadows", "Hidden City", None),
    k("*.FarmVille2CountryEscape", "FarmVille 2", None),
    k("*.CaesarsSlotsFreeCasino", "Caesars Slots", None),
    k("Clipchamp.Clipchamp", "Clipchamp", None),
    // Applis Microsoft facultatives.
    k("Microsoft.BingNews", "Microsoft News", None),
    k("Microsoft.BingWeather", "MSN Weather", None),
    k("Microsoft.BingFinance", "MSN Money", None),
    k("Microsoft.BingSports", "MSN Sports", None),
    k("Microsoft.MicrosoftSolitaireCollection", "Solitaire Collection", None),
    k("Microsoft.Getstarted", "Tips", None),
    k("Microsoft.GetHelp", "Get Help", Some("gethelp")),
    k("Microsoft.WindowsFeedbackHub", "Feedback Hub", None),
    k("Microsoft.WindowsSoundRecorder", "Sound Recorder", None),
    k("Microsoft.WindowsAlarms", "Clock", Some("clock")),
    k("Microsoft.WindowsMaps", "Maps", None),
    k("Microsoft.People", "People", None),
    k("Microsoft.windowscommunicationsapps", "Mail and Calendar", Some("mail")),
    k("Microsoft.OutlookForWindows", "Outlook (new)", Some("mail")),
    k("Microsoft.SkypeApp", "Skype", None),
    k("MicrosoftTeams", "Teams (personal)", None),
    k("MSTeams", "Teams", None),
    k("Microsoft.MicrosoftOfficeHub", "Microsoft 365 (Office)", None),
    k("Microsoft.Office.OneNote", "OneNote", Some("notes")),
    k("Microsoft.MicrosoftStickyNotes", "Sticky Notes", Some("notes")),
    k("Microsoft.MicrosoftJournal", "Journal", Some("notes")),
    k("Microsoft.Todos", "Microsoft To Do", None),
    k("Microsoft.PowerAutomateDesktop", "Power Automate", None),
    k("Microsoft.ZuneMusic", "Media Player", Some("media")),
    k("Microsoft.ZuneVideo", "Movies & TV", None),
    k("Microsoft.YourPhone", "Phone Link", Some("phone")),
    k("Microsoft.Copilot", "Copilot", None),
    k("Microsoft.549981C3F5F10", "Cortana", None),
    k("MicrosoftWindows.Client.WebExperience", "Widgets", Some("widgets")),
    k("Microsoft.MSPaint", "Paint 3D", None),
    k("Microsoft.Microsoft3DViewer", "3D Viewer", None),
    k("Microsoft.Print3D", "Print 3D", None),
    k("Microsoft.MixedReality.Portal", "Mixed Reality Portal", None),
    k("Microsoft.Whiteboard", "Whiteboard", None),
    k("Microsoft.Windows.DevHome", "Dev Home", None),
    k("MicrosoftCorporationII.MicrosoftFamily", "Family Safety", None),
    k("MicrosoftCorporationII.QuickAssist", "Quick Assist", None),
    k("Microsoft.Wallet", "Wallet", None),
    k("Microsoft.Messaging", "Messaging", None),
    k("Microsoft.OneConnect", "Mobile Plans", None),
    k("Microsoft.NetworkSpeedTest", "Network Speed Test", None),
    // Xbox : Game Pass et les jeux du Store s'en servent. Identité Xbox, TCUI et Gaming Services jamais proposés.
    k("Microsoft.GamingApp", "Xbox", Some("xbox")),
    k("Microsoft.XboxApp", "Xbox Console Companion", Some("xbox")),
    k("Microsoft.XboxGamingOverlay", "Xbox Game Bar", Some("gamebar")),
    k("Microsoft.XboxGameOverlay", "Xbox Game Overlay", Some("xbox")),
    k("Microsoft.XboxSpeechToTextOverlay", "Xbox Speech to Text", Some("xbox")),
];

/// Ne doivent jamais correspondre à un motif de `KNOWN` (vérifié par un test).
#[cfg(test)]
const NEVER: &[&str] = &[
    "Microsoft.WindowsStore",
    "Microsoft.StorePurchaseApp",
    "Microsoft.DesktopAppInstaller",
    "Microsoft.Winget.Source",
    "Microsoft.MicrosoftEdge.Stable",
    "Microsoft.Windows.Photos",
    "Microsoft.WindowsCalculator",
    "Microsoft.WindowsTerminal",
    "Microsoft.WindowsNotepad",
    "Microsoft.Paint",
    "Microsoft.ScreenSketch",
    "Microsoft.WindowsCamera",
    "Microsoft.SecHealthUI",
    "Microsoft.XboxIdentityProvider",
    "Microsoft.Xbox.TCUI",
    "Microsoft.GamingServices",
    "Microsoft.HEVCVideoExtension",
    "Microsoft.WebpImageExtension",
    "Microsoft.VCLibs.140.00",
    "Microsoft.BingSearch",
    "MicrosoftWindows.CrossDevice",
    "NVIDIACorp.NVIDIAControlPanel",
    "AdvancedMicroDevicesInc-2.AMDRadeonSoftware",
    "RealtekSemiconductorCorp.RealtekAudioControl",
    "MICROSOFT.MINECRAFTUWP",
];

fn matches(pattern: &str, name: &str) -> bool {
    let (p, n) = (pattern.to_lowercase(), name.to_lowercase());
    match (p.strip_prefix('*'), p.strip_suffix('*')) {
        (Some(rest), _) if rest.ends_with('*') => n.contains(rest.trim_end_matches('*')),
        (Some(end), _) => n.ends_with(end),
        (None, Some(start)) => n.starts_with(start),
        (None, None) => n == p,
    }
}

fn known(name: &str) -> Option<&'static Known> {
    KNOWN.iter().find(|k| matches(k.pattern, name))
}

#[derive(Deserialize, Default)]
#[serde(rename_all(deserialize = "PascalCase"), default)]
struct Package {
    name: String,
    package_full_name: String,
    package_family_name: String,
    install_location: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct App {
    /// Nom complet du paquet (`PackageFullName`) : identifiant pour le retrait.
    pub id: String,
    pub name: String,
    pub package: String,
    /// `PackageFamilyName` : page du Store pour réinstaller.
    pub family: String,
    pub size: u64,
    pub warn: Option<&'static str>,
}

/// Paquets du Store de l'utilisateur courant, retirables et présents dans `KNOWN`. Lecture seule.
pub fn list() -> Result<Vec<App>, String> {
    let (ok, out) = crate::catalog::commands::powershell(
        "$p = Get-AppxPackage | Where-Object { $_.SignatureKind -eq 'Store' -and -not $_.IsFramework -and -not $_.NonRemovable } | \
         Select-Object Name, PackageFullName, PackageFamilyName, InstallLocation; ConvertTo-Json -Compress -InputObject @($p)",
    );
    let json = out.lines().last().unwrap_or("");
    let packages: Vec<Package> = serde_json::from_str(json).map_err(|e| if ok { e.to_string() } else { out.clone() })?;
    let mut apps: Vec<App> = packages
        .into_iter()
        .filter_map(|p| {
            let k = known(&p.name)?;
            let size = if p.install_location.is_empty() { 0 } else { crate::programs::dir_size(Path::new(&p.install_location)) };
            Some(App { id: p.package_full_name, name: k.name.into(), package: p.name, family: p.package_family_name, size, warn: k.warn })
        })
        .collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    Ok(apps)
}

/// Un nom de paquet ne contient que ces caractères : rien d'autre n'arrive jusqu'à PowerShell.
fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || "._-~".contains(c))
}

#[derive(Serialize, Debug)]
pub struct Removed {
    pub id: String,
    pub error: Option<String>,
}

/// Retire des paquets (déjà vérifiés par l'appelant contre la dernière liste) pour l'utilisateur courant. Journalisé.
pub fn remove(ids: &[String], journal_dir: &Path) -> Vec<Removed> {
    ids.iter()
        .map(|id| {
            let error = if safe_id(id) {
                let (ok, out) = crate::catalog::commands::powershell(&format!("$ErrorActionPreference='Stop'; Remove-AppxPackage -Package '{id}'"));
                let mut j = journal::Entry::new("remove-app");
                j.paths = vec![id.clone()];
                j.output = Some(out.clone());
                j.error = (!ok).then(|| out.lines().filter(|l| !l.trim().is_empty()).nth(1).unwrap_or("failed").trim().to_string());
                let _ = journal::append(journal_dir, &j);
                j.error
            } else {
                Some(format!("Refused: {id} is not a package name"))
            };
            Removed { id: id.clone(), error }
        })
        .collect()
}

/// Page du Store pour réinstaller une appli de `KNOWN` (famille `<nom>_<éditeur>`), même retirée depuis la dernière recherche.
pub fn store_uri(family: &str) -> Option<String> {
    let (name, _) = family.rsplit_once('_')?;
    (safe_id(family) && known(name).is_some()).then(|| format!("ms-windows-store://pdp/?PFN={family}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn essential_apps_are_never_proposed() {
        for name in NEVER {
            assert!(known(name).is_none(), "{name} ne doit jamais être proposé");
        }
    }

    #[test]
    fn patterns() {
        assert_eq!(known("king.com.CandyCrushSaga").map(|k| k.name), Some("Candy Crush (King)"));
        assert_eq!(known("BytedancePte.Ltd.TikTok").map(|k| k.name), Some("TikTok"));
        assert_eq!(known("microsoft.bingweather").map(|k| k.name), Some("MSN Weather"), "casse ignorée");
        assert_eq!(known("Microsoft.XboxGamingOverlay").and_then(|k| k.warn), Some("gamebar"));
        assert!(known("Microsoft.BingWeatherBeta").is_none(), "nom exact");
        assert!(matches("*.Duolingo*", "D5EA27B7.Duolingo-LearnLanguagesforFree"));
    }

    #[test]
    fn ids_reaching_powershell_are_plain() {
        assert!(safe_id("Microsoft.BingWeather_4.53.52892.0_x64__8wekyb3d8bbwe"));
        assert!(!safe_id("x'; Remove-Item C:\\ -Recurse; '"));
        assert!(!safe_id(""));
        assert_eq!(store_uri("Microsoft.BingWeather_8wekyb3d8bbwe").unwrap(), "ms-windows-store://pdp/?PFN=Microsoft.BingWeather_8wekyb3d8bbwe");
        assert!(store_uri("Microsoft.WindowsStore_8wekyb3d8bbwe").is_none(), "hors de KNOWN");
        assert!(store_uri("Microsoft.BingWeather").is_none());
    }

    /// Sur la vraie machine : `cargo test --release debloat::tests::real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real() {
        for a in list().unwrap() {
            println!("{:28} {:>12} {} {:?}", a.name, a.size, a.package, a.warn);
        }
    }
}
