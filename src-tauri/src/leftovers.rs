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
const SHARED: &[&str] = &[
    // Outils en ligne de commande et gestionnaires de paquets : installés sans entrée « Programmes ».
    "pnpm", "pnpmcache", "ghcli", "uv", "pypoetry", "volta", "fnm", "deno", "bun", "nvm", "pipx",
    "wsl", "realtek", "synaptics", "elantech", "dell", "hp", "lenovo", "asus", "msi", "chocolatey", "scoop",
    "conexant", "dolby",
    "microsoft", "windows", "packages", "temp", "tmp", "programs", "virtualstore", "crashdumps", "d3dscache",
    "connecteddevicesplatform", "comms", "publishers", "placeholdertilelogofolder", "peerdistrepub", "history",
    "isolatedstorage", "nvidia", "nvidiacorporation", "amd", "intel", "packagecache", "npm", "npmcache", "pip",
    "yarn", "nuget", "google", "mozilla", "apple", "applecomputer", "adobe", "kleaner", "comkleaner", "squirreltemp",
];

/// Dossiers cachés du profil qui appartiennent à une appli précise : (nom, mots du nom de l'appli).
// .android n'y est pas : il a son propre cas (avertissement), car Flutter et d'autres outils s'en servent aussi.
const DOT_OWNERS: &[(&str, &[&str])] = &[
    (".vscode", &["visual studio code"]),
    (".cursor", &["cursor"]),
    (".windsurf", &["windsurf"]),
    (".trae", &["trae"]),
    (".eclipse", &["eclipse"]),
    (".p2", &["eclipse"]),
    (".matplotlib", &["python"]),
    (".spyder-py3", &["spyder"]),
    (".dbus-keyrings", &["git"]),
    (".docker", &["docker"]),
    (".ollama", &["ollama"]),
    (".lmstudio", &["lm studio"]),
    (".virtualbox", &["virtualbox"]),
    (".vagrant.d", &["vagrant"]),
    (".arduinoide", &["arduino"]),
    (".gimp-2.10", &["gimp"]),
];

/// Dossiers « produit » rangés dans le dossier d'un éditeur (partagé : jamais proposé en entier) :
/// (éditeur normalisé, [(début du nom du dossier normalisé, mot qui doit figurer dans le nom d'un programme installé)]).
/// Un dossier de produit n'est proposé que si aucun programme installé ne porte ce mot.
const PRODUCTS: &[(&str, &[(&str, &str)])] = &[
    ("google", &[("androidstudio", "androidstudio")]),
    (
        "jetbrains",
        &[
            ("intellijidea", "intellij"),
            ("ideaic", "intellij"),
            ("ideaiu", "intellij"),
            ("pycharm", "pycharm"),
            ("webstorm", "webstorm"),
            ("phpstorm", "phpstorm"),
            ("clion", "clion"),
            ("goland", "goland"),
            ("rider", "rider"),
            ("rubymine", "rubymine"),
            ("datagrip", "datagrip"),
            ("dataspell", "dataspell"),
            ("appcode", "appcode"),
            ("rustrover", "rustrover"),
            ("fleet", "fleet"),
        ],
    ),
    (
        "adobe",
        &[
            ("premierepro", "premiere"),
            ("aftereffects", "aftereffects"),
            ("photoshop", "photoshop"),
            ("illustrator", "illustrator"),
            ("lightroom", "lightroom"),
            ("indesign", "indesign"),
            ("audition", "audition"),
            ("mediaencoder", "mediaencoder"),
            ("animate", "animate"),
            ("dreamweaver", "dreamweaver"),
            ("incopy", "incopy"),
            ("prelude", "prelude"),
            ("characteranimator", "characteranimator"),
        ],
    ),
];

/// Réglages rangés dans Documents par des outils de développement : (début du nom normalisé, mot du nom du programme).
const DOC_OWNERS: &[(&str, &str)] =
    &[("visualstudio20", "visual studio"), ("sqlservermanagementstudio", "sql server management"), ("iisexpress", "iis express")];

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
    /// Raison de prudence affichée à côté : « android » = le SDK et les émulateurs servent aussi à Flutter, Unity…
    pub warning: Option<&'static str>,
}

/// Le dossier du SDK Android (`%LOCALAPPDATA%\Android`).
fn android_sdk_dir(name: &str) -> bool {
    norm(name) == "android"
}

/// Le dossier `child` d'un éditeur est-il un produit connu ? Renvoie le mot à chercher dans les programmes installés.
/// « Adobe Premiere Pro 2023 » et « Premiere Pro » donnent le même résultat.
fn product_token(vendor: &str, child: &str) -> Option<&'static str> {
    let n = norm(child);
    let n = n.strip_prefix("adobe").unwrap_or(&n);
    let (_, products) = PRODUCTS.iter().find(|(v, _)| *v == norm(vendor))?;
    products.iter().find(|(prefix, _)| n.starts_with(prefix)).map(|(_, token)| *token)
}

/// Idem pour un dossier caché du profil (« .PyCharm2019.3 », « .AndroidStudio4.1 »), quel que soit l'éditeur.
fn dot_product_token(name: &str) -> Option<&'static str> {
    PRODUCTS.iter().find_map(|(vendor, _)| product_token(vendor, name.trim_start_matches('.')))
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

    // Android Studio désinstallé : son SDK et .android restent, mais Flutter, Unity et d'autres outils s'en servent aussi.
    // On les propose quand même, avec un avertissement (l'utilisateur décide), seulement sans Android Studio.
    let studio = programs.iter().any(|p| norm(&p.name).contains("androidstudio"));
    let mut candidates: Vec<(PathBuf, Place, Option<&'static str>)> = Vec::new();
    let local = env("LOCALAPPDATA");
    let app_roots = [local.clone(), env("APPDATA"), local.as_ref().map(|l| PathBuf::from(format!("{}Low", l.display())))];
    for root in app_roots.into_iter().flatten() {
        for p in subdirs(&root) {
            let name = name_of(&p);
            if android_sdk_dir(&name) {
                if !studio {
                    candidates.push((p, Place::AppData, Some("android")));
                }
            } else if PRODUCTS.iter().any(|(v, _)| *v == norm(&name)) && (system(&name) || owned(&name)) {
                // Dossier d'éditeur (Google, Adobe, JetBrains) : jamais proposé en entier, mais ses dossiers de produits,
                // un par un, quand le produit n'est plus installé (par exemple Google/AndroidStudio2025.2).
                for c in subdirs(&p) {
                    let installed = |token: &str| programs.iter().any(|prog| norm(&prog.name).contains(token));
                    if product_token(&name, &name_of(&c)).is_some_and(|token| !installed(token)) && only_generic(&c) {
                        candidates.push((c, Place::AppData, None));
                    }
                }
            } else if !name.starts_with('.') && !system(&name) && only_generic(&p) && !owned(&name) {
                candidates.push((p, Place::AppData, None));
            }
        }
    }
    if let Some(l) = &local {
        for p in subdirs(&l.join("Programs")) {
            let name = name_of(&p);
            if !system(&name) && !owned(&name) && !programs.iter().any(|prog| under(&prog.location, &p)) {
                candidates.push((p, Place::Programs, None));
            }
        }
    }
    // Anciennes versions laissées par une mise à jour (ex. Opera GX : 120.0…, 129.0… à côté de la version en cours).
    if let Some(l) = &local {
        for prog in programs.iter().filter(|p| under(&p.location, l)) {
            for p in old_versions(Path::new(&prog.location), &prog.version) {
                candidates.push((p, Place::Programs, None));
            }
        }
    }
    if let Some(home) = env("USERPROFILE") {
        for (dot, owners) in DOT_OWNERS {
            let p = home.join(dot);
            let installed = programs.iter().any(|prog| owners.iter().any(|o| prog.name.to_lowercase().contains(o)));
            if p.is_dir() && !installed {
                candidates.push((p, Place::Profile, None));
            }
        }
        if !studio {
            let android = home.join(".android");
            if android.is_dir() {
                candidates.push((android, Place::Profile, Some("android")));
            }
        }
        // Anciens dossiers de réglages d'un produit désinstallé (.PyCharm2019.3, .AndroidStudio4.1…).
        for p in subdirs(&home).into_iter().filter(|p| name_of(p).starts_with('.')) {
            let installed = |token: &str| programs.iter().any(|prog| norm(&prog.name).contains(token));
            if dot_product_token(&name_of(&p)).is_some_and(|token| !installed(token)) {
                candidates.push((p, Place::Profile, None));
            }
        }
        // Réglages d'outils de développement rangés dans Documents (Visual Studio 2019, SQL Server Management Studio…).
        for p in subdirs(&home.join("Documents")) {
            let n = norm(&name_of(&p));
            let owner = DOC_OWNERS.iter().find(|(prefix, _)| n.starts_with(prefix)).map(|(_, o)| *o);
            if owner.is_some_and(|o| !programs.iter().any(|prog| prog.name.to_lowercase().contains(o))) {
                candidates.push((p, Place::Profile, None));
            }
        }
    }
    for (var, place) in [("ProgramFiles", Place::ProgramFiles), ("ProgramFiles(x86)", Place::ProgramFiles), ("ProgramData", Place::ProgramData)] {
        let Some(root) = env(var) else { continue };
        for p in subdirs(&root) {
            let name = name_of(&p);
            if !system(&name) && only_generic(&p) && !owned(&name) {
                candidates.push((p, place, None));
            }
        }
    }

    let mut out: Vec<Leftover> = candidates
        .par_iter()
        .filter_map(|(p, place, warning)| {
            let (size, modified) = size_and_newest(p);
            (size >= MIN_SIZE && modified < quiet).then(|| Leftover {
                path: p.display().to_string(),
                name: name_of(p),
                place: *place,
                size,
                modified,
                trashable: matches!(place, Place::AppData | Place::Programs | Place::Profile),
                warning: *warning,
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
    fn vendor_product_folders_are_recognised() {
        assert_eq!(product_token("Google", "AndroidStudio2025.2.2"), Some("androidstudio"));
        assert_eq!(product_token("JetBrains", "PyCharmCE2023.1"), Some("pycharm"));
        assert_eq!(product_token("JetBrains", "IdeaIC2023.2"), Some("intellij"));
        assert_eq!(product_token("Adobe", "Premiere Pro"), Some("premiere"));
        assert_eq!(product_token("Adobe", "Adobe After Effects 2023"), Some("aftereffects"));
        // Infrastructure et données partagées : jamais prises pour un produit.
        assert_eq!(product_token("Adobe", "Common"), None);
        assert_eq!(product_token("Google", "Chrome"), None);
        assert_eq!(product_token("Google", "DriveFS"), None);
        assert_eq!(product_token("JetBrains", "consent"), None);
        assert_eq!(product_token("Mozilla", "Firefox"), None, "éditeur sans liste : rien");
        assert_eq!(dot_product_token(".PyCharm2019.3"), Some("pycharm"));
        assert_eq!(dot_product_token(".AndroidStudio4.1"), Some("androidstudio"));
        assert_eq!(dot_product_token(".ssh"), None);
        assert_eq!(dot_product_token(".gradle"), None);
    }

    #[test]
    fn android_folders_are_recognised() {
        assert!(android_sdk_dir("Android") && android_sdk_dir("android"));
        assert!(!android_sdk_dir("Android Studio") && !android_sdk_dir("androidx"));
    }

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
