//! Catalogue communautaire : entrées de Winapp2.ini (projet Winapp2, licence CC BY-SA 4.0),
//! converties à l'avance en `winapp2.json` (voir `WINAPP2-NOTICE.md`). Aucun réseau à l'exécution.
//!
//! Winapp2 contient aussi des cookies, historiques, mots de passe et clés de registre : tout cela est écarté.
//! On ne garde que les dossiers de cache, de fichiers temporaires, de journaux et de rapports de plantage,
//! dans AppData, ProgramData (admin) ou les dossiers cachés du profil. Le filtre `entry_ok` s'applique aussi au
//! chargement : un `winapp2.json` modifié à la main ne peut pas élargir ce que Kleaner supprime.

use crate::catalog::{Cleaner, Level, Mode, Text};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const JSON: &str = include_str!("../winapp2.json");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: String,
    pub label: String,
    pub paths: Vec<String>,
    #[serde(default)]
    pub only: Vec<String>,
    #[serde(default)]
    pub admin: bool,
    /// Sans RECURSE ni REMOVESELF dans Winapp2 : pas de sous-dossiers.
    #[serde(default)]
    pub flat: bool,
}

#[derive(Serialize, Deserialize)]
struct File {
    source: String,
    license: String,
    version: String,
    entries: Vec<Entry>,
}

/// Mots (en minuscules) qui rendent un chemin ou un titre suspect : données de l'utilisateur, sessions, identifiants.
const DENY: &[&str] = &[
    "cookie", "history", "login", "password", "passwd", "credential", "session", "bookmark", "autofill", "webdata",
    "web data", "formhistory", "local storage", "localstorage", "indexeddb", "service worker", "serviceworker",
    "network", "sync", "extension", "preferences", "wallet", "token", "licen", "backup", "recent", "download",
    "document", "picture", "music", "video", "desktop", "onedrive", "dropbox", "save", "profile", "setting",
    "config", "database", "contact",
    // Cache de Google Drive (modifications pas encore envoyées), fichiers hors ligne, installeurs et paquets (réparation),
    // mods, coffres et comptes.
    "mcafee", "norton", "symantec", "kaspersky", "avast", "avira", "bitdefender", "eset", "malwarebytes", "defender",
    "drivefs", "offline", "installer", "package", "setup", "uninstall", "mods", "vault", "secret", "account", "identity",
];

/// Premiers dossiers sous AppData qui appartiennent à Windows ou au Store : le catalogue intégré s'en charge.
const SYSTEM_FIRST: [&str; 4] = ["microsoft", "packages", "programs", "temp"];

/// Racines acceptées (les mêmes variables que le catalogue intégré).
const ROOTS: [&str; 5] = ["%LOCALAPPDATA%", "%APPDATA%", "%ProgramData%", "%USERPROFILE%\\AppData\\LocalLow", "%USERPROFILE%"];

fn lower(s: &str) -> String {
    s.to_lowercase()
}

fn denied(s: &str) -> bool {
    let l = lower(s);
    DENY.iter().any(|w| l.contains(w))
}

/// Un dossier dont le nom dit qu'il est jetable.
fn comp_positive(c: &str) -> bool {
    let c = lower(c);
    c.contains("cache")
        || c.contains("crash")
        || c.contains("dump")
        || c.contains("thumb")
        || c.contains("shader")
        || matches!(c.as_str(), "temp" | "tmp" | "temporary" | "log" | "logs" | "temporary files")
        || c.ends_with(" logs")
        || c.ends_with(" log")
        || c.ends_with("_logs")
        || c.ends_with("_temp")
        || c.ends_with(" temp")
}

/// Un motif de fichier jetable : `*.log`, `*.tmp`, `*.dmp`…
fn pattern_positive(p: &str) -> bool {
    let p = lower(p);
    let p = p.trim_end_matches('*');
    [".log", ".tmp", ".temp", ".dmp", ".mdmp", ".etl", ".crash"].iter().any(|e| p.ends_with(e))
}

/// Premier mot réel du chemin, après la variable.
fn components(path: &str) -> Vec<&str> {
    path.split('\\').filter(|c| !c.is_empty()).collect()
}

/// Le chemin (déjà converti en `%VAR%\…`) et ses motifs sont-ils acceptables ?
fn path_ok(path: &str, only: &[String]) -> bool {
    let Some(root) = ROOTS.iter().find(|r| path.starts_with(**r) && path[r.len()..].starts_with('\\')) else { return false };
    let rest = &path[root.len() + 1..];
    let comps = components(rest);
    if comps.len() < 2 || path.contains("..") || path.chars().any(char::is_control) || path.contains("\\\\") {
        return false;
    }
    // Profil : seulement AppData\LocalLow et les dossiers cachés (.gradle…), jamais Documents, Bureau…
    if *root == "%USERPROFILE%" && !comps[0].starts_with('.') {
        return false;
    }
    if (*root == "%LOCALAPPDATA%" || *root == "%APPDATA%") && SYSTEM_FIRST.contains(&lower(comps[0]).as_str()) {
        return false;
    }
    if denied(rest) || only.iter().any(|p| denied(p)) {
        return false;
    }
    let last = comps[comps.len() - 1];
    comp_positive(last) || (!only.is_empty() && only.iter().all(|p| pattern_positive(p)))
}

fn entry_ok(e: &Entry) -> bool {
    !e.paths.is_empty()
        && !denied(&e.label)
        && e.id.starts_with("w2-")
        && e.paths.iter().all(|p| path_ok(p, &e.only))
        && e.admin == e.paths.iter().any(|p| p.starts_with("%ProgramData%"))
}

/// Entrées de `winapp2.json` qui passent le filtre, sous forme de nettoyages du catalogue.
pub fn cleaners() -> Vec<Cleaner> {
    let Ok(file) = serde_json::from_str::<File>(JSON) else { return Vec::new() };
    let t = |en: &str, fr: &str| Text { en: en.into(), fr: fr.into() };
    file.entries
        .into_iter()
        .filter(entry_ok)
        .map(|e| Cleaner {
            label: Text { en: e.label.clone(), fr: e.label },
            what: t(
                "Cache, temporary or log files of this program (community catalog Winapp2).",
                "Fichiers de cache, temporaires ou journaux de ce programme (catalogue communautaire Winapp2).",
            ),
            why: t("The program recreates them when it needs them.", "Le programme les recrée quand il en a besoin."),
            effect: t(
                "Nothing important. The next launch can be a little slower.",
                "Rien d’important. Le prochain lancement peut être un peu plus lent.",
            ),
            id: e.id,
            group: "community".into(),
            paths: e.paths,
            only: e.only,
            older_than_hours: 0,
            flat: e.flat,
            admin: e.admin,
            mode: Mode::Delete,
            level: Level::Safe,
            close: Vec::new(),
            command: None,
        })
        .collect()
}

// ---------- Conversion de Winapp2.ini (outil de développement) ----------

struct Raw {
    title: String,
    keys: Vec<String>,
    skip: bool,
}

fn parse(ini: &str) -> Vec<Raw> {
    let mut out: Vec<Raw> = Vec::new();
    for line in ini.lines() {
        let line = line.trim();
        if line.starts_with(';') || line.is_empty() {
            continue;
        }
        if let Some(title) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            out.push(Raw { title: title.trim_end_matches('*').trim().to_string(), keys: Vec::new(), skip: false });
        } else if let (Some((k, v)), Some(cur)) = (line.split_once('='), out.last_mut()) {
            let k = lower(k);
            if k.starts_with("filekey") {
                cur.keys.push(v.to_string());
            } else if k.starts_with("excludekey") || k == "warning" || (k == "default" && lower(v) == "false") {
                // Exclusions non gérées, avertissements, et entrées que CCleaner ne coche pas : on écarte.
                cur.skip = true;
            }
        }
    }
    out
}

/// `%LocalAppData%\App\Cache` → `%LOCALAPPDATA%\App\Cache`. `None` si la racine n'est pas gérée.
fn map_dir(dir: &str) -> Option<String> {
    let d = dir.trim().trim_end_matches('\\');
    let (var, rest) = d.strip_prefix('%')?.split_once('%')?;
    let rest = rest.strip_prefix('\\')?;
    let root = match lower(var).as_str() {
        "localappdata" => "%LOCALAPPDATA%",
        "appdata" => "%APPDATA%",
        "programdata" | "commonappdata" => "%ProgramData%",
        "userprofile" => "%USERPROFILE%",
        "locallowappdata" => "%USERPROFILE%\\AppData\\LocalLow",
        _ => return None,
    };
    Some(format!("{root}\\{rest}"))
}

fn slug(title: &str) -> String {
    let s: String = title.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-")
}

/// Mots des noms de programmes déjà dans le catalogue intégré : leurs doublons Winapp2 sont ignorés.
fn covered_words(base: &[Cleaner]) -> HashSet<String> {
    const GENERIC: [&str; 8] = ["google", "microsoft", "adobe", "games", "launcher", "code", "dart", "flutter"];
    base.iter()
        .flat_map(|c| lower(c.label.en.split(':').next().unwrap_or("")).split(|ch: char| !ch.is_alphanumeric()).map(String::from).collect::<Vec<_>>())
        .filter(|w| w.len() >= 3 && !GENERIC.contains(&w.as_str()))
        .collect()
}

/// Le dossier est-il le même que celui d'une entrée du catalogue intégré, ou contenu dans l'un, ou l'inverse ?
fn overlaps(path: &str, known: &[String]) -> bool {
    let p = lower(path);
    known.iter().any(|k| p == *k || p.starts_with(&format!("{k}\\")) || k.starts_with(&format!("{p}\\")))
}

/// Convertit le contenu de Winapp2.ini. `base` : catalogue intégré (pour éviter les doublons).
pub fn convert(ini: &str, base: &[Cleaner]) -> Vec<Entry> {
    let covered = covered_words(base);
    let known: Vec<String> = base.iter().flat_map(|c| c.paths.iter().map(|p| lower(p))).collect();
    let mut out: Vec<Entry> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();
    for raw in parse(ini).into_iter().filter(|r| !r.skip && !denied(&r.title)) {
        let title_words: Vec<String> = lower(&raw.title).split(|c: char| !c.is_alphanumeric()).map(String::from).collect();
        if title_words.iter().any(|w| covered.contains(w) || (w.len() >= 5 && covered.iter().any(|c| c.len() >= 5 && w.starts_with(c.as_str())))) {
            continue;
        }
        // Un nettoyage par jeu de motifs : (motifs, dossiers).
        let mut groups: Vec<(Vec<String>, bool, Vec<String>)> = Vec::new();
        for key in &raw.keys {
            let mut parts = key.split('|');
            let Some(dir) = parts.next() else { continue };
            // « dossier|RECURSE » : le deuxième champ est un drapeau, pas un motif.
            let second = parts.next().unwrap_or("*.*");
            let second_is_flag = matches!(lower(second).as_str(), "recurse" | "removeself");
            let pats = if second_is_flag { "*.*" } else { second };
            let mut flags: Vec<String> = parts.map(lower).collect();
            if second_is_flag {
                flags.push(lower(second));
            }
            let flat = !flags.iter().any(|f| f == "recurse" || f == "removeself");
            let Some(path) = map_dir(dir) else { continue };
            if overlaps(&path, &known) {
                continue;
            }
            let mut only: Vec<String> = pats.split(';').map(str::trim).filter(|p| !p.is_empty()).map(String::from).collect();
            if only.iter().any(|p| p == "*" || p == "*.*") {
                only.clear();
            }
            only.sort();
            if !path_ok(&path, &only) {
                continue;
            }
            match groups.iter_mut().find(|g| g.0 == only && g.1 == flat) {
                Some(g) => g.2.push(path),
                None => groups.push((only, flat, vec![path])),
            }
        }
        for (n, (only, flat, mut paths)) in groups.into_iter().enumerate() {
            paths.sort();
            paths.dedup();
            let mut id = format!("w2-{}", slug(&raw.title));
            if n > 0 {
                id.push_str(&format!("-{}", n + 1));
            }
            if !used.insert(id.clone()) {
                continue;
            }
            let admin = paths.iter().any(|p| p.starts_with("%ProgramData%"));
            let entry = Entry { id, label: raw.title.clone(), paths, only, admin, flat };
            if entry_ok(&entry) && entry.label.chars().count() <= 100 {
                out.push(entry);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(id: &str, path: &str, only: &[&str]) -> Entry {
        Entry {
            id: id.into(),
            label: "Test".into(),
            paths: vec![path.into()],
            only: only.iter().map(|s| s.to_string()).collect(),
            admin: path.starts_with("%ProgramData%"),
            flat: false,
        }
    }

    #[test]
    fn only_disposable_folders_pass() {
        assert!(entry_ok(&e("w2-a", r"%LOCALAPPDATA%\Vendor\App\Cache", &[])));
        assert!(entry_ok(&e("w2-a", r"%LOCALAPPDATA%\Vendor\App\GPUCache", &[])));
        assert!(entry_ok(&e("w2-a", r"%APPDATA%\Vendor\App", &["*.log"])));
        assert!(entry_ok(&e("w2-a", r"%ProgramData%\Vendor\App\Logs", &[])));
        assert!(entry_ok(&e("w2-a", r"%USERPROFILE%\.tool\cache", &[])));
        assert!(entry_ok(&e("w2-a", r"%USERPROFILE%\AppData\LocalLow\Vendor\Crashes", &[])));
    }

    #[test]
    fn user_data_and_sessions_never_pass() {
        // Dossier non jetable, motif non jetable, mots interdits, racines non gérées, trop large.
        assert!(!entry_ok(&e("w2-a", r"%LOCALAPPDATA%\Vendor\App\Data", &[])));
        assert!(!entry_ok(&e("w2-a", r"%APPDATA%\Vendor\App", &["*.db"])));
        assert!(!entry_ok(&e("w2-a", r"%LOCALAPPDATA%\Google\Chrome\User Data\Default\Cookies", &[])));
        assert!(!entry_ok(&e("w2-a", r"%LOCALAPPDATA%\Google\Chrome\User Data\Default\Cache\Login Data", &[])));
        assert!(!entry_ok(&e("w2-a", r"%LOCALAPPDATA%\Vendor\Sessions\Cache", &[])));
        assert!(!entry_ok(&e("w2-a", r"%LOCALAPPDATA%\Vendor\Local Storage\Cache", &[])));
        assert!(!entry_ok(&e("w2-a", r"%LOCALAPPDATA%\Cache", &[])), "trop large");
        assert!(!entry_ok(&e("w2-a", r"%USERPROFILE%\Documents\Cache", &[])));
        assert!(!entry_ok(&e("w2-a", r"%USERPROFILE%\Desktop\x\Cache", &[])));
        assert!(!entry_ok(&e("w2-a", r"%LOCALAPPDATA%\Microsoft\Windows\INetCache", &[])), "Windows : catalogue intégré");
        assert!(!entry_ok(&e("w2-a", r"%SystemRoot%\Temp\x\Cache", &[])));
        assert!(!entry_ok(&e("w2-a", r"%LOCALAPPDATA%\Vendor\..\Other\Cache", &[])));
        assert!(!entry_ok(&e("w2-a", r"%LOCALAPPDATA%\Vendor\Catalog", &[])), "« log » dans « catalog » ne compte pas");
        assert!(!entry_ok(&e("other-id", r"%LOCALAPPDATA%\Vendor\App\Cache", &[])));
        let mut admin = e("w2-a", r"%ProgramData%\Vendor\App\Cache", &[]);
        admin.admin = false;
        assert!(!entry_ok(&admin), "ProgramData demande admin");
    }

    #[test]
    fn conversion_maps_filters_and_skips_what_it_cannot_honour() {
        let ini = "\
; comment
[Example App Cache *]
LangSecRef=3025
Detect=HKCU\\Software\\Example
FileKey1=%LocalAppData%\\Example\\App\\Cache|*.*|RECURSE
FileKey2=%LocalAppData%\\Example\\App\\Logs|*.log;*.txt
FileKey3=%LocalAppData%\\Example\\App\\Data|*.*
FileKey4=%Documents%\\Example\\Cache|*.*
FileKey5=%LocalAppData%\\Example\\App\\Temp|RECURSE
RegKey1=HKCU\\Software\\Example\\Recent

[Example Cookies *]
FileKey1=%AppData%\\Example\\Cache\\Cookies|*.*

[Risky Cache *]
Default=False
FileKey1=%AppData%\\Risky\\App\\Cache|*.*

[Excluding Cache *]
ExcludeKey1=FILE|%AppData%\\Excluding\\App\\Cache|keep.me
FileKey1=%AppData%\\Excluding\\App\\Cache|*.*

[Chrome Cache *]
FileKey1=%LocalAppData%\\Google\\Chrome\\Cache|*.*
";
        let base = [Cleaner { label: Text { en: "Google Chrome: cache".into(), fr: String::new() }, ..sample() }];
        let got = convert(ini, &base);
        assert_eq!(got.len(), 2, "{got:?}");
        assert_eq!(got[0].id, "w2-example-app-cache");
        let want = [r"%LOCALAPPDATA%\Example\App\Cache", r"%LOCALAPPDATA%\Example\App\Temp"];
        assert_eq!(got[0].paths, want, "« Temp|RECURSE » : drapeau, pas motif");
        assert!(got[0].only.is_empty() && !got[0].flat, "RECURSE : descend");
        assert_eq!(got[1].id, "w2-example-app-cache-2");
        assert_eq!(got[1].only, ["*.log", "*.txt"].map(String::from));
        assert!(got[1].flat, "sans RECURSE : pas de sous-dossiers");
    }

    #[test]
    fn same_folder_as_the_builtin_catalog_is_skipped() {
        let known = vec![r"%localappdata%\pip\cache".to_string()];
        assert!(overlaps(r"%LOCALAPPDATA%\Pip\cache", &known));
        assert!(overlaps(r"%LOCALAPPDATA%\Pip\cache\http", &known));
        assert!(overlaps(r"%LOCALAPPDATA%\Pip", &known));
        assert!(!overlaps(r"%LOCALAPPDATA%\Pipe\cache", &known));
    }

    fn sample() -> Cleaner {
        Cleaner {
            id: "x".into(),
            group: "apps".into(),
            label: Text { en: String::new(), fr: String::new() },
            what: Text { en: String::new(), fr: String::new() },
            why: Text { en: String::new(), fr: String::new() },
            effect: Text { en: String::new(), fr: String::new() },
            paths: Vec::new(),
            only: Vec::new(),
            older_than_hours: 0,
            flat: false,
            admin: false,
            mode: Mode::Delete,
            level: Level::Safe,
            close: Vec::new(),
            command: None,
        }
    }

    #[test]
    fn committed_file_is_fully_accepted() {
        let file: File = serde_json::from_str(JSON).unwrap();
        assert_eq!(file.license, "CC-BY-SA-4.0");
        for e in &file.entries {
            assert!(entry_ok(e), "{} refusé par le filtre", e.id);
        }
        assert_eq!(cleaners().len(), file.entries.len());
    }

    /// Régénère `winapp2.json` : `WINAPP2_INI=<Winapp2.ini> cargo test --release winapp2::tests::regenerate -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn regenerate() {
        let path = std::env::var("WINAPP2_INI").expect("WINAPP2_INI");
        let ini = std::fs::read_to_string(&path).unwrap();
        let version = ini.lines().find_map(|l| l.strip_prefix("; Version: ")).unwrap_or("?").trim().to_string();
        let entries = convert(&ini, crate::catalog::base());
        println!("{} entrées Winapp2 → {} nettoyages", parse(&ini).len(), entries.len());
        let file = File {
            source: "https://github.com/MoscaDotTo/Winapp2".into(),
            license: "CC-BY-SA-4.0".into(),
            version,
            entries,
        };
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("winapp2.json");
        std::fs::write(out, serde_json::to_string_pretty(&file).unwrap() + "\n").unwrap();
    }
}
