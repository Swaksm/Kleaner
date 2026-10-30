//! Moteur du catalogue : tout ce que Kleaner peut supprimer définitivement est décrit dans
//! `cleaners.json`. Ce qui n'y figure pas ne peut pas être supprimé.

use crate::{guard, journal};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, Metadata};
use std::os::windows::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const CATALOG_JSON: &str = include_str!("../cleaners.json");

/// Phrase anglaise → traduction, une table par langue (`texts/<langue>.json`). Phrase absente : l'interface montre l'anglais.
fn translations() -> &'static [(&'static str, HashMap<String, String>)] {
    static T: OnceLock<Vec<(&'static str, HashMap<String, String>)>> = OnceLock::new();
    T.get_or_init(|| {
        [
            ("es", include_str!("../texts/es.json")),
            ("de", include_str!("../texts/de.json")),
            ("it", include_str!("../texts/it.json")),
            ("pt", include_str!("../texts/pt.json")),
            ("ru", include_str!("../texts/ru.json")),
            ("zh", include_str!("../texts/zh.json")),
        ]
        .into_iter()
        .map(|(lang, json)| (lang, serde_json::from_str(json).expect("texts/<langue>.json invalide")))
        .collect()
    })
}

/// Texte affiché : français et anglais écrits à la main, autres langues ajoutées depuis `texts/`.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(from = "TextSource")]
pub struct Text {
    pub fr: String,
    pub en: String,
    /// es, de, it, pt, ru, zh.
    #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub more: BTreeMap<&'static str, String>,
}

#[derive(Deserialize)]
struct TextSource {
    fr: String,
    en: String,
}

impl From<TextSource> for Text {
    fn from(t: TextSource) -> Self {
        Text::new(t.en, t.fr)
    }
}

impl Text {
    pub fn new(en: impl Into<String>, fr: impl Into<String>) -> Self {
        let en = en.into();
        let more = translations().iter().filter_map(|(lang, map)| map.get(&en).map(|s| (*lang, s.clone()))).collect();
        Self { fr: fr.into(), en, more }
    }

    /// La même transformation dans chaque langue.
    pub fn map(&self, f: impl Fn(&str) -> String) -> Self {
        Self { fr: f(&self.fr), en: f(&self.en), more: self.more.iter().map(|(l, s)| (*l, f(s))).collect() }
    }

    /// Remplace `{nom}` par sa valeur dans chaque langue.
    pub fn with(&self, vars: &[(&str, &str)]) -> Self {
        self.map(|s| vars.iter().fold(s.to_string(), |acc, (k, v)| acc.replace(&format!("{{{k}}}"), v)))
    }

    /// Assemble deux textes langue par langue ; une langue absente d'un côté y prend l'anglais.
    pub fn join(&self, other: &Text, f: impl Fn(&str, &str) -> String) -> Self {
        let get = |t: &Text, l: &str| t.more.get(l).unwrap_or(&t.en).clone();
        let langs: Vec<&'static str> = self.more.keys().chain(other.more.keys()).copied().collect();
        Self {
            fr: f(&self.fr, &other.fr),
            en: f(&self.en, &other.en),
            more: langs.into_iter().map(|l| (l, f(&get(self, l), &get(other, l)))).collect(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Suppression du contenu des dossiers `paths`.
    #[default]
    Delete,
    /// Outil Windows officiel, géré par `id` (ex. vider la Corbeille).
    Command,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Coché par défaut, sans effet visible pour l'utilisateur.
    #[default]
    Safe,
    /// Carte dédiée, confirmation en deux étapes.
    Heavy,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
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
    /// Ne descend pas dans les sous-dossiers : seulement les fichiers directement dans chaque dossier de `paths`.
    #[serde(default)]
    pub flat: bool,
    #[serde(default)]
    pub admin: bool,
    pub mode: Mode,
    pub level: Level,
    /// Processus qui doivent être fermés ; sinon l'entrée est ignorée.
    #[serde(default)]
    pub close: Vec<String>,
    /// Commande affichée à l'utilisateur avant exécution (`mode: command`).
    #[serde(default)]
    pub command: Option<String>,
}

/// Le catalogue écrit à la main (`cleaners.json`).
pub fn base() -> &'static [Cleaner] {
    static BASE: OnceLock<Vec<Cleaner>> = OnceLock::new();
    BASE.get_or_init(|| serde_json::from_str(CATALOG_JSON).expect("cleaners.json invalide"))
}

/// Tout ce que Kleaner peut supprimer : le catalogue intégré et le catalogue communautaire Winapp2.
pub fn all() -> &'static [Cleaner] {
    static CATALOG: OnceLock<Vec<Cleaner>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut all = base().to_vec();
        all.extend(crate::winapp2::cleaners());
        all
    })
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
pub(crate) fn expand_env(s: &str) -> Option<String> {
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
    /// Sortie des commandes Windows lancées (DISM, powercfg…).
    pub output: Option<String>,
    pub error: Option<String>,
}

struct Walk<'a> {
    entry: &'a Cleaner,
    roots: &'a [PathBuf],
    cutoff: Option<SystemTime>,
    dry_run: bool,
    report: CleanReport,
    /// Simulation : chemins et tailles de ce qui serait supprimé.
    listed: Option<Vec<(String, u64)>>,
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
                if self.entry.flat {
                    empty = false;
                    continue;
                }
                let sub_empty = self.dir(&p);
                let removable = sub_empty && self.entry.only.is_empty() && !self.dry_run;
                if !(removable && guard::check_delete(&p, self.roots).is_ok() && fs::remove_dir(&p).is_ok()) {
                    empty = false;
                }
            } else if !self.wanted(&p, &m) {
                empty = false;
            } else if self.dry_run || self.remove_file(&p, &m) {
                if let Some(l) = &mut self.listed {
                    l.push((p.display().to_string(), m.len()));
                }
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

/// Liste des processus lancés (sortie de tasklist en minuscules). Un appel suffit pour tout le catalogue.
pub fn process_list() -> String {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .creation_flags(crate::system::CREATE_NO_WINDOW)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase())
        .unwrap_or_default()
}

/// Processus de `names` présents dans `list` (voir `process_list`), noms insensibles à la casse.
pub fn running_in(list: &str, names: &[String]) -> Vec<String> {
    names.iter().filter(|n| list.contains(&format!("\"{}\"", n.to_lowercase()))).cloned().collect()
}

/// Processus de la liste actuellement lancés.
pub fn running(names: &[String]) -> Vec<String> {
    if names.is_empty() {
        return Vec::new();
    }
    running_in(&process_list(), names)
}

fn walk(entry: &Cleaner, dry_run: bool) -> CleanReport {
    if !dry_run {
        let blocked_by = running(&entry.close);
        if !blocked_by.is_empty() {
            return CleanReport { id: entry.id.clone(), blocked_by, ..Default::default() };
        }
    }
    if entry.mode == Mode::Command {
        return commands::run(entry, dry_run);
    }
    walk_paths(entry, dry_run)
}

/// Vide le contenu des dossiers `paths` de l'entrée (ou compte seulement si `dry_run`).
fn walk_paths(entry: &Cleaner, dry_run: bool) -> CleanReport {
    let roots: Vec<PathBuf> = entry.paths.iter().flat_map(|p| resolve(p)).collect();
    walk_roots(entry, &roots, dry_run, false).0
}

fn walk_roots(entry: &Cleaner, roots: &[PathBuf], dry_run: bool, list: bool) -> (CleanReport, Vec<(String, u64)>) {
    let report = CleanReport { id: entry.id.clone(), ..Default::default() };
    let cutoff = (entry.older_than_hours > 0)
        .then(|| SystemTime::now() - Duration::from_secs(entry.older_than_hours * 3600));
    let mut w = Walk { entry, roots, cutoff, dry_run, report, listed: list.then(Vec::new) };
    for r in roots {
        w.dir(r);
    }
    (w.report, w.listed.unwrap_or_default())
}

/// Simulation : la liste exacte des fichiers qu'un nettoyage supprimerait. Ne modifie rien.
/// Les commandes Windows (DISM, powercfg…) n'ont pas de liste : seulement leur estimation.
pub fn preview(entry: &Cleaner) -> (CleanReport, Vec<(String, u64)>) {
    if entry.mode == Mode::Command && entry.paths.is_empty() {
        return (estimate(entry), Vec::new());
    }
    let roots: Vec<PathBuf> = entry.paths.iter().flat_map(|p| resolve(p)).collect();
    walk_roots(entry, &roots, true, true)
}

/// Supprime définitivement des dossiers régénérables de projets (node_modules, target…),
/// trouvés par l'analyse. Même garde que le catalogue : liens jamais suivis, fichiers verrouillés ignorés.
pub fn delete_project_dirs(dirs: &[PathBuf], journal_dir: &Path) -> Result<CleanReport, String> {
    let entry = Cleaner { id: "project-artifacts".into(), group: "dev".into(), ..Default::default() };
    let (mut report, _) = walk_roots(&entry, dirs, false, false);
    // Le dossier lui-même, une fois vide (la garde exige d'être strictement sous une racine : son parent).
    for d in dirs {
        let parent: Vec<PathBuf> = d.parent().map(Path::to_path_buf).into_iter().collect();
        if guard::check_delete(d, &parent).is_ok() {
            let _ = fs::remove_dir(d);
        }
    }
    let mut j = journal::Entry::new("clean:project-artifacts");
    j.paths = dirs.iter().map(|d| d.display().to_string()).collect();
    j.files = report.files;
    j.bytes = report.bytes;
    j.skipped = report.skipped;
    journal::append(journal_dir, &j).map_err(|e| format!("Log: {e}"))?;
    report.id = "project-artifacts".into();
    Ok(report)
}

/// Taille et nombre de fichiers qui seraient supprimés. Ne modifie rien.
pub fn estimate(entry: &Cleaner) -> CleanReport {
    walk(entry, true)
}

/// Supprime et journalise. Refuse une entrée admin si Kleaner n'est pas élevé.
pub fn execute(entry: &Cleaner, journal_dir: &Path) -> Result<CleanReport, String> {
    if entry.admin && !crate::elevation::is_elevated() {
        return Err("Admin rights required: run the full scan.".into());
    }
    let mut report = walk(entry, false);
    if !report.blocked_by.is_empty() {
        report.error = Some(format!("Close first: {}", report.blocked_by.join(", ")));
    }
    let mut j = journal::Entry::new(format!("clean:{}", entry.id));
    j.paths = entry.paths.clone();
    j.files = report.files;
    j.bytes = report.bytes;
    j.skipped = report.skipped;
    j.error = report.error.clone();
    j.output = report.output.clone();
    journal::append(journal_dir, &j).map_err(|e| format!("Log: {e}"))?;
    Ok(report)
}

/// Entrées `mode: command`, une fonction par `id`. Uniquement des outils Windows officiels.
mod commands {
    use super::{CleanReport, Cleaner, resolve, walk_paths};
    use crate::guard;
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use windows_sys::Win32::UI::Shell::{
        SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHEmptyRecycleBinW, SHQUERYRBINFO, SHQueryRecycleBinW,
    };

    pub const IDS: [&str; 9] = [
        "recycle-bin",
        "memory-dump",
        "delivery-optimization",
        "windows-update-cache",
        "windows-old",
        "component-store",
        "hibernation",
        "restore-points",
        "compact-os",
    ];

    fn env_path(var: &str, rest: &str) -> Option<PathBuf> {
        std::env::var_os(var).map(|v| PathBuf::from(v).join(rest))
    }

    /// Lance un programme sans fenêtre et renvoie sa sortie (stdout + stderr).
    fn exec(program: &str, args: &[&str]) -> (bool, String) {
        match std::process::Command::new(program).args(args).creation_flags(crate::system::CREATE_NO_WINDOW).output() {
            Ok(o) => {
                let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                (o.status.success(), format!("> {program} {}\n{}", args.join(" "), text.trim()))
            }
            Err(e) => (false, format!("> {program} {}\n{e}", args.join(" "))),
        }
    }

    fn powershell(script: &str) -> (bool, String) {
        let script = format!("[Console]::OutputEncoding=[Text.Encoding]::UTF8; {script}");
        exec("powershell.exe", &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
    }

    fn file_size(p: Option<PathBuf>) -> u64 {
        p.and_then(|p| std::fs::metadata(p).ok()).map_or(0, |m| m.len())
    }

    /// Taille actuelle (avant nettoyage) de ce que la commande retirerait.
    fn measure(entry: &Cleaner) -> CleanReport {
        let mut r = CleanReport { id: entry.id.clone(), ..Default::default() };
        match entry.id.as_str() {
            "recycle-bin" => {
                let mut info = SHQUERYRBINFO { cbSize: size_of::<SHQUERYRBINFO>() as u32, i64Size: 0, i64NumItems: 0 };
                // Racine nulle = toutes les Corbeilles de tous les disques.
                if unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) } == 0 {
                    r.bytes = info.i64Size as u64;
                    r.files = info.i64NumItems as u64;
                }
            }
            "memory-dump" => {
                r.bytes = file_size(env_path("SystemRoot", "MEMORY.DMP"));
                r.files = u64::from(r.bytes > 0);
            }
            "hibernation" => {
                r.bytes = file_size(env_path("SystemDrive", "\\hiberfil.sys"));
                r.files = u64::from(r.bytes > 0);
            }
            "restore-points" => {
                let (_, out) = powershell(
                    "$s=@(Get-CimInstance Win32_ShadowCopy); $u=(Get-CimInstance Win32_ShadowStorage | Measure-Object UsedSpace -Sum).Sum; \"$($s.Count) $([int64]$u)\"",
                );
                let nums: Vec<u64> = out.lines().last().unwrap_or("").split(' ').filter_map(|n| n.parse().ok()).collect();
                if let [count, used] = nums[..] {
                    // On garde le plus récent : estimation proportionnelle.
                    r.files = count.saturating_sub(1);
                    r.bytes = if count > 1 { used / count * (count - 1) } else { 0 };
                }
            }
            // Taille connue seulement après DISM ou compact.
            "component-store" | "compact-os" => {}
            _ => r = walk_paths(entry, true),
        }
        r
    }

    pub fn run(entry: &Cleaner, dry_run: bool) -> CleanReport {
        let before = measure(entry);
        if dry_run || !IDS.contains(&entry.id.as_str()) {
            return before;
        }
        let mut r = CleanReport { id: entry.id.clone(), ..Default::default() };
        // (succès, sortie) de chaque étape ; les étapes informatives sont marquées réussies.
        let mut steps: Vec<(bool, String)> = Vec::new();
        let mut check_result = true;
        // compact ne supprime rien : le gain se lit dans l'espace libre du disque système.
        let mut freed = None;
        match entry.id.as_str() {
            "recycle-bin" => {
                if before.files > 0 {
                    let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
                    let hr = unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), flags) };
                    steps.push((hr == 0, format!("SHEmptyRecycleBinW -> {hr:#x}")));
                }
            }
            "memory-dump" => {
                if let (Some(file), Some(root)) = (env_path("SystemRoot", "MEMORY.DMP"), std::env::var_os("SystemRoot")) {
                    let res = guard::check_delete(&file, &[PathBuf::from(root)])
                        .map_err(|e| e.to_string())
                        .and_then(|f| std::fs::remove_file(f).map_err(|e| e.to_string()));
                    steps.push((res.is_ok(), format!("delete {}: {res:?}", file.display())));
                }
            }
            "delivery-optimization" => steps.push(powershell("Delete-DeliveryOptimizationCache -Force")),
            "windows-update-cache" => {
                steps.push(powershell("Stop-Service wuauserv,bits -Force"));
                let w = walk_paths(entry, false);
                r.skipped = w.skipped;
                // Relancé dans tous les cas, même si la suppression a échoué.
                steps.push(powershell("Start-Service wuauserv,bits"));
            }
            "windows-old" => {
                for root in entry.paths.iter().flat_map(|p| resolve(p)) {
                    let p = root.display().to_string();
                    // /D attend « Y » en anglais et « O » en français : l'option inconnue échoue sans effet.
                    steps.push(exec("takeown", &["/F", &p, "/R", "/A", "/D", "Y"]));
                    steps.push((true, exec("takeown", &["/F", &p, "/R", "/A", "/D", "O"]).1));
                    steps.push(exec("icacls", &[&p, "/grant", "*S-1-5-32-544:F", "/T", "/C", "/Q"]));
                }
                let w = walk_paths(entry, false);
                r.skipped = w.skipped;
                // Restent des dossiers vides et des jonctions : rd ne suit pas les jonctions.
                for root in entry.paths.iter().flat_map(|p| resolve(p)) {
                    steps.push((true, exec("cmd", &["/C", "rd", "/S", "/Q", &root.display().to_string()]).1));
                }
                check_result = false; // le résultat réel est mesuré ci-dessous
            }
            "component-store" => {
                let dism = env_path("SystemRoot", "System32\\Dism.exe").unwrap_or_else(|| "Dism.exe".into());
                steps.push(exec(&dism.display().to_string(), &["/Online", "/Cleanup-Image", "/StartComponentCleanup"]));
            }
            "hibernation" => steps.push(exec("powercfg", &["/hibernate", "off"])),
            "compact-os" => {
                let root = env_path("SystemDrive", "\\");
                let free = || root.as_deref().and_then(crate::scan::drive_space).map(|s| s.free);
                let start = free();
                steps.push(exec("compact", &["/CompactOS:always"]));
                freed = start.zip(free()).map(|(a, b)| b.saturating_sub(a));
            }
            "restore-points" => steps.push(powershell(
                "$s=@(Get-CimInstance Win32_ShadowCopy | Sort-Object InstallDate); if ($s.Count -gt 1) { $s | Select-Object -First ($s.Count-1) | Remove-CimInstance }; \"$($s.Count) point(s), most recent one kept\"",
            )),
            _ => {}
        }
        let after = measure(entry);
        r.bytes = freed.unwrap_or(before.bytes.saturating_sub(after.bytes));
        r.files = before.files.saturating_sub(after.files);
        r.output = Some(steps.iter().map(|s| s.1.as_str()).collect::<Vec<_>>().join("\n\n"));
        if check_result && steps.iter().any(|s| !s.0) {
            r.error = Some("A command failed, see the output.".into());
        }
        r
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
        Text::new("x", "x")
    }

    #[test]
    fn text_helpers_apply_to_every_language() {
        // Sans passer par Text::new : ce test ne dépend pas des fichiers texts/.
        let raw = |en: &str, fr: &str, lang: &'static str, tr: &str| Text { en: en.into(), fr: fr.into(), more: [(lang, tr.to_string())].into() };
        let a = raw("Inside {dir}.", "Dans {dir}.", "de", "In {dir}.");
        let b = raw("Cache.", "Cache.", "es", "Caché.");
        let j = a.with(&[("dir", "X")]).join(&b, |x, y| format!("{x} {y}"));
        assert_eq!((j.en.as_str(), j.fr.as_str()), ("Inside X. Cache.", "Dans X. Cache."));
        assert_eq!(j.more["de"], "In X. Cache.", "allemand d'un côté, anglais de l'autre");
        assert_eq!(j.more["es"], "Inside X. Caché.");
        let json = serde_json::to_value(&j).unwrap();
        assert_eq!(json["de"], "In X. Cache.", "les autres langues sont à plat, à côté de fr et en");
        // Le catalogue lu depuis cleaners.json reçoit les traductions de texts/.
        let temp = &base().iter().find(|c| c.id == "user-temp").unwrap().label;
        assert_eq!((temp.more["de"].as_str(), temp.more["zh"].as_str()), ("Temporäre Dateien", "临时文件"));
    }

    #[test]
    fn translation_files_are_valid() {
        for (lang, map) in translations() {
            for (en, tr) in map {
                assert!(!tr.trim().is_empty(), "{lang} : traduction vide pour « {en} »");
                // Les gabarits gardent leurs {valeurs} et leurs {1} de capture.
                let marks = |s: &str| s.match_indices('{').map(|(i, _)| s[i..].split('}').next().unwrap_or("").to_string()).collect::<std::collections::BTreeSet<_>>();
                assert_eq!(marks(en), marks(tr), "{lang} : « {en} »");
            }
        }
    }

    fn cleaner(paths: Vec<String>) -> Cleaner {
        Cleaner { id: "test".into(), group: "test".into(), label: text(), what: text(), why: text(), effect: text(), paths, ..Default::default() }
    }

    fn journal_dir(s: &Sandbox) -> PathBuf {
        s.0.join("journal")
    }

    #[test]
    fn preview_lists_without_deleting() {
        let s = Sandbox::new("preview");
        let a = s.file("cache/a.bin", 100);
        s.file("cache/sub/b.bin", 50);
        let (r, list) = preview(&cleaner(vec![s.0.join("cache").display().to_string()]));
        assert_eq!((r.files, r.bytes, list.len()), (2, 150, 2));
        assert!(a.exists());
    }

    #[test]
    fn project_dirs_deleted_with_folder_and_logged() {
        let s = Sandbox::new("projdel");
        s.file("web/package.json", 10);
        s.file("web/node_modules/x/y.js", 100);
        let nm = s.0.join("web").join("node_modules");
        let r = delete_project_dirs(std::slice::from_ref(&nm), &journal_dir(&s)).unwrap();
        assert_eq!((r.files, r.bytes), (1, 100));
        assert!(!nm.exists(), "node_modules supprimé");
        assert!(s.0.join("web").join("package.json").exists(), "le projet reste");
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

    /// Winapp2 sans RECURSE : seulement le dossier lui-même (un `.log` de LevelDB dans un sous-dossier reste).
    #[test]
    fn flat_never_descends_into_subfolders() {
        let s = Sandbox::new("flat");
        let top = s.file("app/debug.log", 10);
        let deep = s.file("app/Local Storage/leveldb/000003.log", 10);
        let mut c = cleaner(vec![s.0.join("app").display().to_string()]);
        c.only = vec!["*.log".into()];
        c.flat = true;
        let r = execute(&c, &journal_dir(&s)).unwrap();
        assert_eq!(r.files, 1);
        assert!(!top.exists() && deep.exists());
        c.flat = false;
        execute(&c, &journal_dir(&s)).unwrap();
        assert!(!deep.exists(), "sans flat, la descente reste le comportement normal");
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

    const ALLOWED_VARS: [&str; 8] = [
        "%TEMP%",
        "%LOCALAPPDATA%",
        "%APPDATA%",
        "%USERPROFILE%",
        "%SystemRoot%",
        "%ProgramData%",
        "%SystemDrive%",
        "%ProgramFiles(x86)%",
    ];
    const ADMIN_VARS: [&str; 3] = ["%SystemRoot%", "%ProgramData%", "%SystemDrive%"];
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
                    assert!(!s.chars().any(char::is_control), "{} : {name} contient un caractère de contrôle", c.id);
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
                assert!(rest.is_empty() || rest.starts_with('\\'), "{} : {p} : « \\ » manquant après la variable", c.id);
                assert!(!p.chars().any(char::is_control), "{} : {p:?} contient un caractère de contrôle", c.id);
                assert!(!rest.contains("\\\\"), "{} : {p} : « \\\\ » en double", c.id);
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
    fn command_entries_are_implemented_and_heavy_needs_admin() {
        for c in all() {
            if c.mode == Mode::Command {
                assert!(commands::IDS.contains(&c.id.as_str()), "{} : commande non implémentée", c.id);
                assert!(c.command.is_some(), "{} : commande à afficher manquante", c.id);
                let cmd = c.command.as_deref().unwrap_or("");
                // Un « % » suivi d'une lettre sans « \ » = variable collée au chemin (échappement perdu).
                assert!(!cmd.contains("%SystemRoot%M") && !cmd.contains("SoftwareDistributionDownload"), "{} : {cmd}", c.id);
                assert!(!cmd.chars().any(char::is_control), "{} : caractère de contrôle", c.id);
            }
            if c.level == Level::Heavy && c.id != "recycle-bin" {
                assert!(c.admin, "{} : heavy système sans admin", c.id);
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
