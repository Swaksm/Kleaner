//! Restes de programmes désinstallés : dossiers d'AppData qui n'appartiennent plus à aucun programme.
//! Lecture seule. Un dossier n'est proposé que s'il n'est rattaché à rien :
//! ni à un programme installé (nom, éditeur, dossier), ni à une règle connue, ni à un dossier partagé de Windows,
//! et qu'il n'a pas été modifié depuis 30 jours.

use crate::knowledge::{self, Source};
use crate::programs::{Program, version_key};
use rayon::prelude::*;
use serde::Serialize;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MIN_SIZE: u64 = 10 * 1024 * 1024;
const QUIET_DAYS: u64 = 30;

/// Dossiers partagés ou de Windows, jamais proposés (noms normalisés).
// « android » : le SDK sert aussi à Flutter et à d'autres outils non inscrits dans Programmes.
const SHARED: [&str; 48] = [
    "android", "wsl", "realtek", "synaptics", "elantech", "dell", "hp", "lenovo", "asus", "msi", "chocolatey", "scoop",
    "conexant", "dolby",
    "microsoft", "windows", "packages", "temp", "tmp", "programs", "virtualstore", "crashdumps", "d3dscache",
    "connecteddevicesplatform", "comms", "publishers", "placeholdertilelogofolder", "peerdistrepub", "history",
    "isolatedstorage", "nvidia", "nvidiacorporation", "amd", "intel", "packagecache", "npm", "npmcache", "pip",
    "yarn", "nuget", "google", "mozilla", "apple", "applecomputer", "adobe", "kleaner", "comkleaner", "squirreltemp",
];

/// Dossiers cachés du profil qui appartiennent à une appli précise : (nom, mots du nom de l'appli).
// .android n'y est pas : Flutter et d'autres outils utilisent aussi les émulateurs Android.
const DOT_OWNERS: [(&str, &[&str]); 7] = [
    (".vscode", &["visual studio code"]),
    (".cursor", &["cursor"]),
    (".windsurf", &["windsurf"]),
    (".eclipse", &["eclipse"]),
    (".p2", &["eclipse"]),
    (".matplotlib", &["python"]),
    (".dbus-keyrings", &["git"]),
];

/// Composants de Windows sans entrée « Programmes » : jamais proposés (préfixes normalisés).
const SYSTEM_PREFIXES: [&str; 12] = [
    "windows", "microsoft", "commonfiles", "internetexplorer", "referenceassemblies", "msbuild", "dotnet",
    "modifiablewindowsapps", "uninstallinformation", "regid", "uso", "iis",
];

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    AppData,
    Programs,
    Profile,
    ProgramFiles,
    ProgramData,
}

#[derive(Serialize, Clone, Debug)]
pub struct Leftover {
    pub path: String,
    pub name: String,
    pub place: Place,
    pub size: u64,
    /// Dernière modification d'un fichier dedans (secondes Unix).
    pub modified: u64,
    /// Envoi à la Corbeille possible. Program Files et ProgramData : signalés seulement (droits admin, garde).
    pub trashable: bool,
}

/// Minuscules, lettres et chiffres seulement.
fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// « Arduino15 » → « arduino » : les numéros de version ne disent rien du propriétaire.
fn stem(s: &str) -> String {
    norm(s).trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}

/// `dir` est-il le dossier d'installation ou un de ses parents ?
fn under(location: &str, dir: &Path) -> bool {
    let (l, d) = (location.to_lowercase(), dir.display().to_string().to_lowercase());
    !l.is_empty() && (l == d || l.starts_with(&format!("{d}\\")))
}

/// Le dossier `name` peut-il appartenir à ce programme ?
fn owned_by(name: &str, p: &Program) -> bool {
    let n = stem(name);
    if n.len() < 3 {
        return true; // trop court pour conclure : on ne propose pas
    }
    let (pn, pp) = (norm(&p.name), norm(&p.publisher));
    pn.contains(&n) || (pp.len() >= 3 && (pp.contains(&n) || n.contains(&pp))) || (pn.len() >= 4 && n.contains(&pn))
}

fn size_and_newest(dir: &Path) -> (u64, u64) {
    let Ok(rd) = std::fs::read_dir(dir) else { return (0, 0) };
    let mut size = 0;
    let mut newest = 0;
    let mut subdirs = Vec::new();
    for e in rd.flatten() {
        let Ok(m) = e.metadata() else { continue };
        if m.is_dir() {
            if m.file_attributes() & 0x400 == 0 {
                subdirs.push(e.path());
            }
        } else {
            size += m.len();
            let t = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
            newest = newest.max(t);
        }
    }
    let children: Vec<(u64, u64)> = subdirs.par_iter().map(|d| size_and_newest(d)).collect();
    (size + children.iter().map(|c| c.0).sum::<u64>(), children.iter().map(|c| c.1).fold(newest, u64::max))
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.metadata().is_ok_and(|m| m.is_dir() && m.file_attributes() & 0x400 == 0))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default()
}

/// « 129.0.5823.70 » : au moins trois nombres séparés par des points.
fn is_version(name: &str) -> bool {
    let parts: Vec<&str> = name.split('.').collect();
    parts.len() >= 3 && parts.iter().all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}

/// Sous-dossiers « numéro de version » d'un dossier d'installation, sauf la version installée et la plus récente.
fn old_versions(location: &Path, current: &str) -> Vec<PathBuf> {
    let mut versions: Vec<(Vec<u64>, PathBuf)> = subdirs(location)
        .into_iter()
        .filter_map(|p| {
            let n = p.file_name()?.to_string_lossy().to_string();
            is_version(&n).then(|| (version_key(&n), p))
        })
        .collect();
    versions.sort_by(|a, b| a.0.cmp(&b.0));
    versions.pop();
    let current = version_key(current);
    versions.into_iter().filter(|(v, _)| *v != current).map(|(_, p)| p).collect()
}

fn env(k: &str) -> Option<PathBuf> {
    std::env::var_os(k).map(PathBuf::from)
}

/// Dossiers orphelins, du plus gros au plus petit :
/// AppData (Local, Roaming, LocalLow), AppData\Local\Programs, dossiers cachés du profil liés à une appli,
/// et (signalés seulement) Program Files et ProgramData.
pub fn find(programs: &[Program]) -> Vec<Leftover> {
    let ctx = knowledge::Context::new(programs.to_vec());
    let quiet = SystemTime::now() - Duration::from_secs(QUIET_DAYS * 86_400);
    let quiet = quiet.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let name_of = |p: &Path| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let owned = |name: &str| programs.iter().any(|prog| owned_by(name, prog));
    // Rattaché à une règle précise, au catalogue ou au dossier d'un programme installé : pas un reste.
    let only_generic = |p: &Path| {
        let e = ctx.explain(&p.display().to_string());
        e.source == Source::Rule && e.rule.as_deref().is_some_and(|r| knowledge::rules().iter().any(|k| k.id == r && k.generic))
    };
    let system = |name: &str| {
        let n = norm(name);
        SHARED.contains(&n.as_str()) || SYSTEM_PREFIXES.iter().any(|p| n.starts_with(p))
    };

    let mut candidates: Vec<(PathBuf, Place)> = Vec::new();
    let local = env("LOCALAPPDATA");
    let app_roots = [local.clone(), env("APPDATA"), local.as_ref().map(|l| PathBuf::from(format!("{}Low", l.display())))];
    for root in app_roots.into_iter().flatten() {
        for p in subdirs(&root) {
            let name = name_of(&p);
            if !name.starts_with('.') && !system(&name) && only_generic(&p) && !owned(&name) {
                candidates.push((p, Place::AppData));
            }
        }
    }
    if let Some(l) = &local {
        for p in subdirs(&l.join("Programs")) {
            let name = name_of(&p);
            if !system(&name) && !owned(&name) && !programs.iter().any(|prog| under(&prog.location, &p)) {
                candidates.push((p, Place::Programs));
            }
        }
    }
    // Anciennes versions laissées par une mise à jour (ex. Opera GX : 120.0…, 129.0… à côté de la version en cours).
    if let Some(l) = &local {
        for prog in programs.iter().filter(|p| under(&p.location, l)) {
            for p in old_versions(Path::new(&prog.location), &prog.version) {
                candidates.push((p, Place::Programs));
            }
        }
    }
    if let Some(home) = env("USERPROFILE") {
        for (dot, owners) in DOT_OWNERS {
            let p = home.join(dot);
            let installed = programs.iter().any(|prog| owners.iter().any(|o| prog.name.to_lowercase().contains(o)));
            if p.is_dir() && !installed {
                candidates.push((p, Place::Profile));
            }
        }
    }
    for (var, place) in [("ProgramFiles", Place::ProgramFiles), ("ProgramFiles(x86)", Place::ProgramFiles), ("ProgramData", Place::ProgramData)] {
        let Some(root) = env(var) else { continue };
        for p in subdirs(&root) {
            let name = name_of(&p);
            if !system(&name) && only_generic(&p) && !owned(&name) {
                candidates.push((p, place));
            }
        }
    }

    let mut out: Vec<Leftover> = candidates
        .par_iter()
        .filter_map(|(p, place)| {
            let (size, modified) = size_and_newest(p);
            (size >= MIN_SIZE && modified < quiet).then(|| Leftover {
                path: p.display().to_string(),
                name: name_of(p),
                place: *place,
                size,
                modified,
                trashable: matches!(place, Place::AppData | Place::Programs | Place::Profile),
            })
        })
        .collect();
    out.sort_by_key(|l| std::cmp::Reverse(l.size));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_version_folders_except_installed_and_latest() {
        let root = std::env::temp_dir().join("kleaner-old-versions");
        let _ = std::fs::remove_dir_all(&root);
        for d in ["120.0.5543.85", "129.0.5823.70", "136.0.6008.111", "136.0.6008.76", "Assets", "autoupdate"] {
            std::fs::create_dir_all(root.join(d)).unwrap();
        }
        let mut old: Vec<String> =
            old_versions(&root, "136.0.6008.76").iter().map(|p| p.file_name().unwrap().to_string_lossy().into()).collect();
        old.sort();
        assert_eq!(old, ["120.0.5543.85", "129.0.5823.70"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn prog(name: &str, publisher: &str) -> Program {
        Program {
            id: "x".into(),
            name: name.into(),
            publisher: publisher.into(),
            version: String::new(),
            bytes: 0,
            install_date: String::new(),
            location: String::new(),
        }
    }

    #[test]
    fn ownership_by_name_publisher_and_version_suffix() {
        assert!(owned_by("Arduino15", &prog("Arduino IDE", "Arduino SA")));
        assert!(owned_by("Riot Games", &prog("League of Legends", "Riot Games, Inc")));
        assert!(owned_by("Discord", &prog("Discord", "Discord Inc.")));
        assert!(owned_by("ab", &prog("Zzz", "Yyy")), "nom trop court : jamais proposé");
        assert!(!owned_by("Arduino15", &prog("7-Zip 24.08 (x64)", "Igor Pavlov")));
    }

    #[test]
    fn shared_folders_never_proposed() {
        assert!(SHARED.contains(&norm("Microsoft").as_str()));
        assert!(SHARED.contains(&norm("Package Cache").as_str()));
    }

    /// Sur la vraie machine : `cargo test --release leftovers::tests::real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_machine() {
        let t = std::time::Instant::now();
        let list = find(&crate::programs::list());
        println!("{} restes en {} ms", list.len(), t.elapsed().as_millis());
        for l in list.iter().take(25) {
            println!("{:>8.0} Mo  {}", l.size as f64 / 1048576.0, l.path);
        }
    }
}
