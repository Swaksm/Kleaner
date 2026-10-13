//! Moteur du catalogue : tout ce que Kleaner peut supprimer définitivement est décrit dans
//! `cleaners.json`. Ce qui n'y figure pas ne peut pas être supprimé.

use crate::{guard, journal};
use serde::{Deserialize, Serialize};
use std::fs::{self, Metadata};
use std::os::windows::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const CATALOG_JSON: &str = include_str!("../cleaners.json");

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Text {
    pub fr: String,
    pub en: String,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Suppression du contenu des dossiers `paths`.
    Delete,
    /// Outil Windows officiel, géré par `id` (ex. vider la Corbeille).
    Command,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Coché par défaut, sans effet visible pour l'utilisateur.
    Safe,
    /// Carte dédiée, confirmation en deux étapes.
    Heavy,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Cleaner {
    pub id: String,
    pub group: String,
    pub label: Text,
    /// Ce que ça supprime.
    pub what: Text,
    /// Pourquoi c'est inutile.
    pub why: Text,
    /// Ce que ça change pour l'utilisateur.
    pub effect: Text,
    #[serde(default)]
    pub paths: Vec<String>,
    /// Si non vide, seuls les fichiers dont le nom correspond sont supprimés (jokers `*` `?`).
    #[serde(default)]
    pub only: Vec<String>,
    /// Ne supprime que les fichiers plus vieux que ça (ex. temporaires d'un installeur en cours).
    #[serde(default)]
    pub older_than_hours: u64,
    #[serde(default)]
    pub admin: bool,
    pub mode: Mode,
    pub level: Level,
    /// Processus qui doivent être fermés ; sinon l'entrée est ignorée.
    #[serde(default)]
    pub close: Vec<String>,
}

pub fn all() -> &'static [Cleaner] {
    static CATALOG: OnceLock<Vec<Cleaner>> = OnceLock::new();
    CATALOG.get_or_init(|| serde_json::from_str(CATALOG_JSON).expect("cleaners.json invalide"))
}

pub fn get(id: &str) -> Option<&'static Cleaner> {
    all().iter().find(|c| c.id == id)
}

/// `*` et `?`, insensible à la casse.
fn wildcard(pattern: &str, name: &str) -> bool {
    let (p, n): (Vec<char>, Vec<char>) = (pattern.to_lowercase().chars().collect(), name.to_lowercase().chars().collect());
    let (mut pi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

/// Remplace `%VAR%` par sa valeur. `None` si une variable n'existe pas.
fn expand_env(s: &str) -> Option<String> {
    let mut out = String::new();
    let mut parts = s.split('%');
    out.push_str(parts.next()?);
    while let Some(var) = parts.next() {
        out.push_str(&std::env::var(var).ok()?);
        out.push_str(parts.next().unwrap_or(""));
    }
    Some(out)
}

fn is_link(m: &Metadata) -> bool {
    m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

/// Dossiers réels correspondant à un chemin du catalogue (variables + jokers), sans traverser de lien.
pub fn resolve(pattern: &str) -> Vec<PathBuf> {
    let Some(expanded) = expand_env(pattern) else { return Vec::new() };
    let mut found = vec![PathBuf::new()];
    for comp in Path::new(&expanded).components() {
        let part = comp.as_os_str().to_string_lossy();
        let is_glob = matches!(comp, Component::Normal(_)) && part.contains(['*', '?']);
        found = found
            .into_iter()
            .flat_map(|base| -> Vec<PathBuf> {
                if !is_glob {
                    return vec![base.join(comp)];
                }
                let Ok(rd) = fs::read_dir(&base) else { return Vec::new() };
                rd.flatten()
                    .filter(|e| e.metadata().is_ok_and(|m| m.is_dir() && !is_link(&m)))
                    .filter(|e| wildcard(&part, &e.file_name().to_string_lossy()))
                    .map(|e| e.path())
                    .collect()
            })
            .collect();
    }
    found
        .into_iter()
        .filter(|p| fs::symlink_metadata(p).is_ok_and(|m| m.is_dir() && !is_link(&m)))
        .collect()
}

#[derive(Serialize, Debug, Default, Clone, PartialEq)]
pub struct CleanReport {
    pub id: String,
    pub files: u64,
    pub bytes: u64,
    /// Fichiers verrouillés, refusés par la garde ou illisibles.
    pub skipped: u64,
    /// Processus à fermer avant de pouvoir nettoyer.
    pub blocked_by: Vec<String>,
}

struct Walk<'a> {
    entry: &'a Cleaner,
    roots: &'a [PathBuf],
    cutoff: Option<SystemTime>,
    dry_run: bool,
    report: CleanReport,
}

impl Walk<'_> {
    fn wanted(&self, p: &Path, m: &Metadata) -> bool {
        let name = p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
        let name_ok = self.entry.only.is_empty() || self.entry.only.iter().any(|pat| wildcard(pat, &name));
        let age_ok = self.cutoff.is_none_or(|c| m.modified().is_ok_and(|t| t < c));
        name_ok && age_ok
    }

    fn remove_file(&self, p: &Path, m: &Metadata) -> bool {
        if guard::check_delete(p, self.roots).is_err() {
            return false;
        }
        if fs::remove_file(p).is_ok() {
            return true;
        }
        // Fichier en lecture seule (fréquent dans les caches de paquets) : on retire l'attribut.
        let mut perms = m.permissions();
        if !perms.readonly() {
            return false;
        }
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        fs::set_permissions(p, perms).is_ok() && fs::remove_file(p).is_ok()
    }

    /// Nettoie le contenu de `dir`. Renvoie `true` si le dossier est vide ensuite.
    // shortcut: garde vérifiée fichier par fichier (sûr mais lent sur 100 k+ fichiers) ; vérifier par dossier si c'est trop lent.
    fn dir(&mut self, dir: &Path) -> bool {
        let Ok(rd) = fs::read_dir(dir) else {
            self.report.skipped += 1;
            return false;
        };
        let mut empty = true;
        for e in rd.flatten() {
            let p = e.path();
            let Ok(m) = e.metadata() else {
                empty = false;
                continue;
            };
            if is_link(&m) {
                empty = false; // jamais toucher un lien ni ce vers quoi il pointe
            } else if m.is_dir() {
                let sub_empty = self.dir(&p);
                let removable = sub_empty && self.entry.only.is_empty() && !self.dry_run;
                if !(removable && guard::check_delete(&p, self.roots).is_ok() && fs::remove_dir(&p).is_ok()) {
                    empty = false;
                }
            } else if !self.wanted(&p, &m) {
                empty = false;
            } else if self.dry_run || self.remove_file(&p, &m) {
                self.report.files += 1;
                self.report.bytes += m.len();
                empty &= !self.dry_run;
            } else {
                self.report.skipped += 1;
                empty = false;
            }
        }
        empty
    }
}

/// Processus de la liste actuellement lancés (noms insensibles à la casse).
pub fn running(names: &[String]) -> Vec<String> {
    if names.is_empty() {
        return Vec::new();
    }
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let Ok(out) = out else { return Vec::new() };
    let list = String::from_utf8_lossy(&out.stdout).to_lowercase();
    names.iter().filter(|n| list.contains(&format!("\"{}\"", n.to_lowercase()))).cloned().collect()
}

fn walk(entry: &Cleaner, dry_run: bool) -> CleanReport {
    let mut report = CleanReport { id: entry.id.clone(), ..Default::default() };
    if entry.mode == Mode::Command {
        return commands::run(entry, dry_run);
    }
    if !dry_run {
        report.blocked_by = running(&entry.close);
        if !report.blocked_by.is_empty() {
            return report;
        }
    }
    let roots: Vec<PathBuf> = entry.paths.iter().flat_map(|p| resolve(p)).collect();
    let cutoff = (entry.older_than_hours > 0)
        .then(|| SystemTime::now() - Duration::from_secs(entry.older_than_hours * 3600));
    let mut w = Walk { entry, roots: &roots, cutoff, dry_run, report };
    for r in &roots {
        w.dir(r);
    }
    w.report
}

/// Taille et nombre de fichiers qui seraient supprimés. Ne modifie rien.
pub fn estimate(entry: &Cleaner) -> CleanReport {
    walk(entry, true)
}

/// Supprime et journalise.
pub fn execute(entry: &Cleaner, journal_dir: &Path) -> Result<CleanReport, String> {
    let report = walk(entry, false);
    let mut j = journal::Entry::new(format!("clean:{}", entry.id));
    j.paths = entry.paths.clone();
    j.files = report.files;
    j.bytes = report.bytes;
    j.skipped = report.skipped;
    if !report.blocked_by.is_empty() {
        j.error = Some(format!("À fermer d'abord : {}", report.blocked_by.join(", ")));
    }
    journal::append(journal_dir, &j).map_err(|e| format!("Journal : {e}"))?;
    Ok(report)
}

/// Entrées `mode: command`, une fonction par `id`.
mod commands {
    use super::{CleanReport, Cleaner};
    use windows_sys::Win32::UI::Shell::{
        SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHEmptyRecycleBinW, SHQUERYRBINFO, SHQueryRecycleBinW,
    };

    pub fn run(entry: &Cleaner, dry_run: bool) -> CleanReport {
        let mut r = CleanReport { id: entry.id.clone(), ..Default::default() };
        // Une commande inconnue ne fait rien. Commandes système : T14/T15.
        if entry.id == "recycle-bin" {
            recycle_bin(&mut r, dry_run);
        }
        r
    }

    fn recycle_bin(r: &mut CleanReport, dry_run: bool) {
        let mut info = SHQUERYRBINFO { cbSize: size_of::<SHQUERYRBINFO>() as u32, i64Size: 0, i64NumItems: 0 };
        // Racine nulle = toutes les Corbeilles de tous les disques.
        if unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) } == 0 {
            r.bytes = info.i64Size as u64;
            r.files = info.i64NumItems as u64;
        }
        if !dry_run && r.files > 0 {
            let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
            if unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), flags) } != 0 {
                r.skipped = r.files;
                r.files = 0;
                r.bytes = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::os::windows::fs::OpenOptionsExt;
    use std::process::Command;

    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("kleaner-catalog-{}-{name}", std::process::id()));
            let _ = Command::new("cmd").args(["/C", "rmdir", "/S", "/Q"]).arg(&dir).status();
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self, rel: &str, size: usize) -> PathBuf {
            let p = self.0.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, vec![0u8; size]).unwrap();
            p
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = Command::new("cmd").args(["/C", "rmdir", "/S", "/Q"]).arg(&self.0).status();
        }
    }

    fn text() -> Text {
        Text { fr: "x".into(), en: "x".into() }
    }

    fn cleaner(paths: Vec<String>) -> Cleaner {
        Cleaner {
            id: "test".into(),
            group: "test".into(),
            label: text(),
            what: text(),
            why: text(),
            effect: text(),
            paths,
            only: vec![],
            older_than_hours: 0,
            admin: false,
            mode: Mode::Delete,
            level: Level::Safe,
            close: vec![],
        }
    }

    fn journal_dir(s: &Sandbox) -> PathBuf {
        s.0.join("journal")
    }

    #[test]
    fn wildcard_matches() {
        assert!(wildcard("thumbcache_*.db", "thumbcache_1024.db"));
        assert!(wildcard("*", "Profile 1"));
        assert!(wildcard("a?c", "ABC"));
        assert!(!wildcard("thumbcache_*.db", "iconcache_32.db"));
        assert!(!wildcard("*.db", "a.dbx"));
    }

    #[test]
    fn expand_env_vars() {
        let temp = std::env::var("TEMP").unwrap();
        assert_eq!(expand_env(r"%TEMP%\x").unwrap(), format!(r"{temp}\x"));
        assert_eq!(expand_env(r"C:\plain").unwrap(), r"C:\plain");
        assert!(expand_env(r"%KLEANER_NOPE_VAR%\x").is_none());
    }

    #[test]
    fn resolve_expands_profile_globs() {
        let s = Sandbox::new("glob");
        fs::create_dir_all(s.0.join("User Data/Default/Cache")).unwrap();
        fs::create_dir_all(s.0.join("User Data/Profile 1/Cache")).unwrap();
        fs::create_dir_all(s.0.join("User Data/Crashpad")).unwrap();
        let mut got = resolve(&format!(r"{}\User Data\*\Cache", s.0.display()));
        got.sort();
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got[0].ends_with(r"Default\Cache"));
    }

    #[test]
    fn execute_empties_root_but_keeps_it() {
        let s = Sandbox::new("execute");
        let root = s.0.join("cache");
        s.file("cache/a.bin", 100);
        s.file("cache/sub/deep/b.bin", 50);
        let c = cleaner(vec![root.display().to_string()]);
        assert_eq!(estimate(&c).bytes, 150);
        let r = execute(&c, &journal_dir(&s)).unwrap();
        assert_eq!((r.files, r.bytes, r.skipped), (2, 150, 0));
        assert!(root.exists(), "la racine reste");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0, "contenu vidé");
        let log = journal::read_last(&journal_dir(&s), 1).unwrap();
        assert_eq!((log[0].action.as_str(), log[0].bytes), ("clean:test", 150));
    }

    #[test]
    fn junction_inside_root_never_followed() {
        let s = Sandbox::new("junction");
        let precious = s.file("docs/precious.txt", 10);
        s.file("cache/a.bin", 10);
        let ok = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(s.0.join("cache").join("trap"))
            .arg(s.0.join("docs"))
            .output()
            .unwrap();
        assert!(ok.status.success());
        let r = execute(&cleaner(vec![s.0.join("cache").display().to_string()]), &journal_dir(&s)).unwrap();
        assert_eq!(r.files, 1);
        assert!(precious.exists(), "la cible de la jonction est intacte");
    }

    #[test]
    fn locked_file_skipped_and_counted() {
        let s = Sandbox::new("locked");
        let f = s.file("cache/locked.bin", 10);
        s.file("cache/free.bin", 10);
        let _hold = File::options().read(true).share_mode(0).open(&f).unwrap();
        let r = execute(&cleaner(vec![s.0.join("cache").display().to_string()]), &journal_dir(&s)).unwrap();
        assert_eq!((r.files, r.skipped), (1, 1));
        assert!(f.exists());
    }

    #[test]
    fn readonly_file_removed() {
        let s = Sandbox::new("readonly");
        let f = s.file("cache/ro.bin", 10);
        let mut p = fs::metadata(&f).unwrap().permissions();
        p.set_readonly(true);
        fs::set_permissions(&f, p).unwrap();
        let r = execute(&cleaner(vec![s.0.join("cache").display().to_string()]), &journal_dir(&s)).unwrap();
        assert_eq!(r.files, 1);
        assert!(!f.exists());
    }

    #[test]
    fn only_and_age_filters() {
        let s = Sandbox::new("filters");
        let keep = s.file("cache/keep.txt", 10);
        let fresh = s.file("cache/thumbcache_new.db", 10);
        let old = s.file("cache/thumbcache_old.db", 10);
        File::options()
            .write(true)
            .open(&old)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(48 * 3600))
            .unwrap();
        let mut c = cleaner(vec![s.0.join("cache").display().to_string()]);
        c.only = vec!["thumbcache_*.db".into()];
        c.older_than_hours = 24;
        let r = execute(&c, &journal_dir(&s)).unwrap();
        assert_eq!(r.files, 1);
        assert!(keep.exists() && fresh.exists() && !old.exists());
    }

    #[test]
    fn running_process_blocks_entry() {
        let s = Sandbox::new("running");
        s.file("cache/a.bin", 10);
        let me = std::env::current_exe().unwrap().file_name().unwrap().to_string_lossy().to_string();
        let mut c = cleaner(vec![s.0.join("cache").display().to_string()]);
        c.close = vec![me.clone()];
        let r = execute(&c, &journal_dir(&s)).unwrap();
        assert_eq!((r.files, r.blocked_by.clone()), (0, vec![me]));
        assert!(journal::read_last(&journal_dir(&s), 1).unwrap()[0].error.is_some());
    }

    // ---- Validation du vrai catalogue ----

    const ALLOWED_VARS: [&str; 6] = ["%TEMP%", "%LOCALAPPDATA%", "%APPDATA%", "%USERPROFILE%", "%SystemRoot%", "%ProgramData%"];
    const ADMIN_VARS: [&str; 2] = ["%SystemRoot%", "%ProgramData%"];
    /// Ce qui garde l'utilisateur connecté : jamais dans un chemin du catalogue.
    const FORBIDDEN: [&str; 11] = [
        "cookies",
        "local storage",
        "indexeddb",
        "login data",
        "session storage",
        "sessions",
        "service worker",
        "network",
        "web data",
        "history",
        "extensions",
    ];

    #[test]
    fn catalog_is_valid() {
        let cat = all();
        assert!(!cat.is_empty());
        let mut ids: Vec<&str> = cat.iter().map(|c| c.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), cat.len(), "ids en double");
        for c in cat {
            for (name, t) in [("label", &c.label), ("what", &c.what), ("why", &c.why), ("effect", &c.effect)] {
                for s in [&t.fr, &t.en] {
                    assert!(!s.trim().is_empty(), "{} : {name} vide", c.id);
                    assert!(s.chars().count() <= 140, "{} : {name} > 140 caractères", c.id);
                }
            }
            if c.mode == Mode::Delete {
                assert!(!c.paths.is_empty(), "{} : aucun chemin", c.id);
            }
        }
    }

    #[test]
    fn catalog_paths_are_narrow_and_admin_flagged() {
        for c in all() {
            for p in &c.paths {
                let var = ALLOWED_VARS.iter().find(|v| p.starts_with(**v));
                let var = var.unwrap_or_else(|| panic!("{} : {p} doit commencer par une variable autorisée", c.id));
                let rest = &p[var.len()..];
                // Seul %TEMP% peut être vidé en entier : ailleurs, au moins un sous-dossier.
                assert!(*var == "%TEMP%" || rest.trim_matches('\\').len() > 1, "{} : {p} trop large", c.id);
                assert!(!p.contains(".."), "{} : {p}", c.id);
                if ADMIN_VARS.contains(var) {
                    assert!(c.admin, "{} : {p} demande admin: true", c.id);
                }
            }
        }
    }

    #[test]
    fn catalog_never_targets_sessions_or_logins() {
        for c in all() {
            for p in &c.paths {
                for comp in p.to_lowercase().split('\\') {
                    assert!(!FORBIDDEN.contains(&comp), "{} : {p} touche « {comp} »", c.id);
                }
            }
        }
    }

    /// Estimation sur la vraie machine : `cargo test --release catalog::tests::estimate_real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn estimate_real() {
        for c in all() {
            let r = estimate(c);
            println!("{:<28} {:>10.1} Mo  {:>7} fichiers", c.id, r.bytes as f64 / 1048576.0, r.files);
        }
    }
}
