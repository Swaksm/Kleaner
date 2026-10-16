//! Programmes installés, lus dans le registre (lecture seule).
//! La désinstallation lance uniquement le désinstalleur déclaré par le programme.

use crate::journal;
use rayon::prelude::*;
use std::collections::HashMap;
use serde::Serialize;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};

const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

/// (préfixe d'id, ruche, vue du registre)
const SOURCES: [(&str, winreg::HKEY, u32); 3] = [
    ("HKLM64", HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY),
    ("HKLM32", HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY),
    ("HKCU", HKEY_CURRENT_USER, 0),
];

#[derive(Serialize, Debug, Clone)]
pub struct Program {
    /// Identifiant stable : source + nom de la clé. L'interface ne renvoie que ça.
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: String,
    /// Taille déclarée par le programme (peut être absente ou fausse).
    pub bytes: u64,
    /// AAAAMMJJ ou vide.
    pub install_date: String,
    /// Dossier d'installation détecté (vide si inconnu ou jugé non fiable).
    pub location: String,
}

fn open(prefix: &str) -> Option<RegKey> {
    let (_, hive, view) = SOURCES.iter().find(|(p, ..)| *p == prefix)?;
    RegKey::predef(*hive).open_subkey_with_flags(UNINSTALL, KEY_READ | view).ok()
}

fn read(prefix: &str, root: &RegKey, key_name: &str) -> Option<Program> {
    let k = root.open_subkey(key_name).ok()?;
    let s = |v: &str| k.get_value::<String, _>(v).unwrap_or_default();
    let d = |v: &str| k.get_value::<u32, _>(v).unwrap_or(0);
    let name = s("DisplayName");
    // Composants système, mises à jour et entrées sans désinstalleur : masqués.
    if name.trim().is_empty()
        || s("UninstallString").trim().is_empty()
        || d("SystemComponent") == 1
        || !s("ParentKeyName").is_empty()
        || matches!(s("ReleaseType").as_str(), "Update" | "Hotfix" | "Security Update")
    {
        return None;
    }
    Some(Program {
        id: format!("{prefix}\\{key_name}"),
        name: name.trim().to_string(),
        publisher: s("Publisher"),
        version: s("DisplayVersion"),
        bytes: u64::from(d("EstimatedSize")) * 1024,
        install_date: s("InstallDate"),
        location: location(&s("InstallLocation"), &s("DisplayIcon"), &s("UninstallString"))
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    })
}

/// Dossiers jamais mesurés en entier : ce ne sont pas des dossiers d'un seul programme.
fn shared_roots() -> Vec<PathBuf> {
    let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let mut v: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)", "ProgramData", "LOCALAPPDATA", "APPDATA", "USERPROFILE"]
        .iter()
        .filter_map(|k| var(k))
        .collect();
    if let Some(local) = var("LOCALAPPDATA") {
        v.push(local.join("Programs"));
    }
    if let Some(home) = var("USERPROFILE").and_then(|h| h.parent().map(Path::to_path_buf)) {
        v.push(home);
    }
    v
}

/// Racines sous lesquelles on accepte un dossier déduit de l'exe (pas déclaré par le programme).
fn install_roots() -> Vec<PathBuf> {
    let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
    ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"].iter().filter_map(|k| var(k)).collect()
}

fn under(child: &Path, parent: &Path) -> bool {
    let (c, p) = (child.to_string_lossy().to_lowercase(), parent.to_string_lossy().to_lowercase());
    let p = p.trim_end_matches('\\');
    c.len() > p.len() + 1 && c.starts_with(p) && c.as_bytes()[p.len()] == b'\\'
}

/// Un dossier mesurable : existe, n'est pas une racine de disque, ni Windows, ni un dossier partagé.
fn usable(dir: &Path, shared: &[PathBuf]) -> bool {
    let lower = dir.to_string_lossy().to_lowercase();
    let windows = std::env::var_os("SystemRoot").map(PathBuf::from);
    dir.is_absolute()
        && dir.is_dir()
        && dir.parent().is_some_and(|p| p.parent().is_some())
        && !shared.iter().any(|r| r.to_string_lossy().eq_ignore_ascii_case(&dir.to_string_lossy()))
        && !windows.is_some_and(|w| under(dir, &w) || w.to_string_lossy().eq_ignore_ascii_case(&dir.to_string_lossy()))
        && !lower.contains("\\package cache\\")
        && !lower.ends_with("\\package cache")
}

fn clean_path(raw: &str) -> PathBuf {
    // DisplayIcon vaut parfois "C:\x\app.exe,0".
    let raw = raw.trim().trim_matches('"');
    let raw = raw.rsplit_once(',').filter(|(_, i)| i.trim().parse::<i32>().is_ok()).map_or(raw, |(p, _)| p);
    PathBuf::from(raw.trim().trim_matches('"').trim_end_matches('\\'))
}

/// Dossier d'installation : `InstallLocation` si fiable, sinon le dossier de l'icône ou du
/// désinstalleur, seulement s'il est sous Program Files ou %LOCALAPPDATA%.
fn location(install: &str, icon: &str, uninstall: &str) -> Option<PathBuf> {
    let shared = shared_roots();
    if !install.trim().is_empty() {
        let p = clean_path(install);
        if usable(&p, &shared) {
            return Some(p);
        }
    }
    let roots = install_roots();
    let from_exe = |raw: &str| -> Option<PathBuf> {
        let exe = if raw.trim_start().starts_with('"') || raw.to_ascii_lowercase().contains(".exe") {
            split_command(raw).map(|(e, _)| e).unwrap_or_default()
        } else {
            raw.to_string()
        };
        let dir = clean_path(&exe).parent()?.to_path_buf();
        (usable(&dir, &shared) && roots.iter().any(|r| under(&dir, r))).then_some(dir)
    };
    from_exe(icon).or_else(|| from_exe(uninstall))
}

/// Taille réelle d'un dossier, sans suivre les liens ni compter les fichiers « en ligne uniquement ».
pub fn dir_size(dir: &Path) -> u64 {
    const REPARSE: u32 = 0x400;
    const RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    let mut files = 0;
    let mut subdirs = Vec::new();
    for e in rd.flatten() {
        let Ok(m) = e.metadata() else { continue };
        let a = m.file_attributes();
        if m.is_dir() {
            if a & REPARSE == 0 {
                subdirs.push(e.path());
            }
        } else if a & RECALL_ON_DATA_ACCESS == 0 {
            files += m.len();
        }
    }
    files + subdirs.par_iter().map(|d| dir_size(d)).sum::<u64>()
}

#[derive(Serialize, Debug, Clone)]
pub struct RealSize {
    pub id: String,
    pub bytes: u64,
    /// Dernier lancement connu (secondes Unix), 0 si inconnu. Lu dans Prefetch : admin uniquement.
    pub last_used: u64,
}

/// Prefetch tronque le nom de l'exe à 29 caractères : « NOM.EXE-HASH.pf ».
const PREFETCH_NAME_MAX: usize = 29;

/// Nom d'exe (majuscules, tronqué) → dernier lancement, d'après C:\Windows\Prefetch.
/// Vide sans droits admin (dossier illisible) ou si Prefetch est désactivé.
fn prefetch_launches() -> HashMap<String, u64> {
    let mut out = HashMap::new();
    let Some(dir) = std::env::var_os("SystemRoot").map(|r| PathBuf::from(r).join("Prefetch")) else { return out };
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_uppercase();
        let Some(stem) = name.strip_suffix(".PF").and_then(|n| n.rsplit_once('-')).map(|(exe, _)| exe.to_string()) else {
            continue;
        };
        let t = e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
        let secs = t.map_or(0, |d| d.as_secs());
        let slot = out.entry(stem).or_insert(0);
        *slot = (*slot).max(secs);
    }
    out
}

/// Noms des .exe d'un dossier d'installation (3 niveaux), au format Prefetch.
fn exe_names(dir: &Path, depth: u32, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(m) = e.metadata() else { continue };
        if m.is_dir() && depth > 0 && m.file_attributes() & 0x400 == 0 {
            exe_names(&e.path(), depth - 1, out);
        } else if m.is_file() {
            let name = e.file_name().to_string_lossy().to_uppercase();
            if name.ends_with(".EXE") {
                out.push(name.chars().take(PREFETCH_NAME_MAX).collect());
            }
        }
    }
}

/// Taille réelle et dernier lancement de chaque programme dont le dossier est connu. Lecture seule.
pub fn real_sizes(programs: &[Program]) -> Vec<RealSize> {
    let launches = prefetch_launches();
    programs
        .par_iter()
        .filter(|p| !p.location.is_empty())
        .map(|p| {
            let dir = Path::new(&p.location);
            let last_used = if launches.is_empty() {
                0
            } else {
                let mut exes = Vec::new();
                exe_names(dir, 3, &mut exes);
                exes.iter().filter_map(|e| launches.get(e)).copied().max().unwrap_or(0)
            };
            RealSize { id: p.id.clone(), bytes: dir_size(dir), last_used }
        })
        .collect()
}

/// « 136.0.6008.111 » → [136, 0, 6008, 111], pour comparer des versions.
pub fn version_key(v: &str) -> Vec<u64> {
    v.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect()
}

/// Tous les programmes, triés par taille décroissante, sans doublons : même nom + version, ou
/// même désinstalleur dans le même dossier (entrée périmée laissée par une mise à jour,
/// ex. Opera GX) — on garde la version la plus récente.
pub fn list() -> Vec<Program> {
    let mut out: Vec<(Program, String)> = Vec::new();
    for (prefix, ..) in SOURCES {
        let Some(root) = open(prefix) else { continue };
        for key_name in root.enum_keys().flatten() {
            let Some(p) = read(prefix, &root, &key_name) else { continue };
            let cmd = root
                .open_subkey(&key_name)
                .and_then(|k| k.get_value::<String, _>("UninstallString"))
                .unwrap_or_default()
                .trim()
                .to_lowercase();
            let same = out.iter().position(|(o, c)| {
                (o.name == p.name && o.version == p.version) || (!p.location.is_empty() && o.location == p.location && *c == cmd)
            });
            match same {
                Some(i) if version_key(&p.version) > version_key(&out[i].0.version) => out[i] = (p, cmd),
                Some(_) => {}
                None => out.push((p, cmd)),
            }
        }
    }
    let mut out: Vec<Program> = out.into_iter().map(|(p, _)| p).collect();
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    out
}

// ---------- Entrées fantômes ----------

/// Entrée « Programmes » dont le désinstalleur et le dossier d'installation n'existent plus.
#[derive(Serialize, Debug, Clone)]
pub struct Ghost {
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub command: String,
}

fn gone(p: &str) -> bool {
    matches!(std::fs::symlink_metadata(p), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
}

/// Le programme lancé par cette commande n'existe plus ? Vrai seulement si on est sûr : chemin absolu complet,
/// sans variable d'environnement, sur un disque présent, et introuvable.
pub fn exe_missing(command: &str) -> bool {
    let Some((exe, _)) = split_command(command) else { return false };
    let b = exe.as_bytes();
    let absolute = b.len() >= 4 && b[1] == b':' && b[2] == b'\\' && !exe.contains('%') && Path::new(&exe[..3]).exists();
    absolute && gone(&exe)
}

/// Désinstalleur introuvable (voir `exe_missing`) et dossier d'installation déclaré absent ou inexistant.
fn is_ghost(command: &str, install_location: &str) -> bool {
    let loc = install_location.trim().trim_matches('"');
    exe_missing(command) && (loc.is_empty() || (!loc.contains('%') && gone(loc)))
}

pub fn ghosts() -> Vec<Ghost> {
    let mut out = Vec::new();
    for (prefix, ..) in SOURCES {
        let Some(root) = open(prefix) else { continue };
        for key_name in root.enum_keys().flatten() {
            let Some(p) = read(prefix, &root, &key_name) else { continue };
            let Ok(k) = root.open_subkey(&key_name) else { continue };
            let command: String = k.get_value("UninstallString").unwrap_or_default();
            let install: String = k.get_value("InstallLocation").unwrap_or_default();
            if is_ghost(&command, &install) {
                out.push(Ghost { id: p.id, name: p.name, version: p.version, publisher: p.publisher, command });
            }
        }
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

fn reg_exe(args: &[&str]) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("reg").args(args).creation_flags(0x0800_0000).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// (ruche lisible par reg.exe, option de vue)
fn reg_target(prefix: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match prefix {
        "HKLM64" => ("HKLM", Some("/reg:64")),
        "HKLM32" => ("HKLM", Some("/reg:32")),
        "HKCU" => ("HKCU", None),
        _ => return None,
    })
}

/// Sauvegarde la clé dans un fichier `.reg`, puis la supprime. Une ligne de journal par entrée, avec le fichier.
pub fn remove_ghost(id: &str, backup_dir: &Path, journal_dir: &Path) -> Result<String, String> {
    let (prefix, key_name) = id.split_once('\\').ok_or("Invalid id")?;
    let (hive, view) = reg_target(prefix).ok_or("Invalid id")?;
    let (_, hkey, flags) = SOURCES.iter().find(|(p, ..)| *p == prefix).ok_or("Invalid id")?;
    // Dernière vérification : l'entrée est toujours un fantôme.
    let root = open(prefix).ok_or("Entry not found")?;
    let k = root.open_subkey(key_name).map_err(|_| "Entry not found")?;
    let command: String = k.get_value("UninstallString").unwrap_or_default();
    let install: String = k.get_value("InstallLocation").unwrap_or_default();
    if !is_ghost(&command, &install) {
        return Err(format!("Refused: {key_name} is not a ghost entry any more"));
    }
    std::fs::create_dir_all(backup_dir).map_err(|e| e.to_string())?;
    let safe: String = key_name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).take(40).collect();
    let file = backup_dir.join(format!("ghost-{prefix}-{safe}-{}.reg", journal::Entry::new("x").ts));
    let key_path = format!("{hive}\\{UNINSTALL}\\{key_name}");
    let mut args = vec!["export", key_path.as_str(), file.to_str().ok_or("Bad path")?, "/y"];
    args.extend(view);
    reg_exe(&args).map_err(|e| format!("Backup failed, nothing removed: {e}"))?;

    let result = RegKey::predef(*hkey)
        .open_subkey_with_flags(UNINSTALL, winreg::enums::KEY_ALL_ACCESS | flags)
        .and_then(|u| u.delete_subkey_all(key_name))
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                "Access denied: relaunch Kleaner as administrator".to_string()
            } else {
                e.to_string()
            }
        });
    let mut j = journal::Entry::new("ghost-remove");
    j.paths = vec![id.to_string(), command];
    j.files = u64::from(result.is_ok());
    j.output = Some(file.display().to_string());
    j.error = result.as_ref().err().cloned();
    journal::append(journal_dir, &j).map_err(|e| format!("Log: {e}"))?;
    result.map(|()| file.display().to_string())
}

/// Remet une entrée supprimée par Kleaner : seulement un fichier `ghost-*.reg` du dossier de sauvegardes.
pub fn restore_ghost(file: &str, backup_dir: &Path, journal_dir: &Path) -> Result<(), String> {
    let path = Path::new(file).canonicalize().map_err(|e| format!("{file}: {e}"))?;
    let dir = backup_dir.canonicalize().map_err(|e| e.to_string())?;
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if path.parent() != Some(dir.as_path()) || !name.starts_with("ghost-") || !name.ends_with(".reg") {
        return Err(format!("Refused: {file} is not a Kleaner backup"));
    }
    let (_, view) = reg_target(name.split('-').nth(1).unwrap_or("")).ok_or("Unknown backup")?;
    let path_str = path.to_str().ok_or("Bad path")?.trim_start_matches(r"\\?\").to_string();
    let mut args = vec!["import", path_str.as_str()];
    args.extend(view);
    let result = reg_exe(&args);
    let mut j = journal::Entry::new("ghost-restore");
    j.paths = vec![file.to_string()];
    j.files = u64::from(result.is_ok());
    j.error = result.as_ref().err().cloned();
    journal::append(journal_dir, &j).map_err(|e| format!("Log: {e}"))?;
    result
}

/// Sépare une ligne de commande de désinstallation en (programme, arguments).
/// `MsiExec /I{GUID}` (réparer) devient `/X{GUID}` (désinstaller).
pub fn split_command(cmd: &str) -> Option<(String, String)> {
    let cmd = cmd.trim();
    let (program, args) = if let Some(rest) = cmd.strip_prefix('"') {
        let end = rest.find('"')?;
        (rest[..end].to_string(), rest[end + 1..].trim().to_string())
    } else {
        // Chemin non entre guillemets, parfois avec des espaces : on coupe après « .exe ».
        let lower = cmd.to_ascii_lowercase();
        match lower.find(".exe") {
            Some(i) => (cmd[..i + 4].to_string(), cmd[i + 4..].trim().to_string()),
            None => match cmd.split_once(' ') {
                Some((p, a)) => (p.to_string(), a.trim().to_string()),
                None => (cmd.to_string(), String::new()),
            },
        }
    };
    if program.is_empty() {
        return None;
    }
    let is_msi = Path::new(&program).file_stem().is_some_and(|s| s.eq_ignore_ascii_case("msiexec"));
    let args = if is_msi && args.to_ascii_uppercase().starts_with("/I") { format!("/X{}", &args[2..]) } else { args };
    Some((program, args))
}

/// Lance le désinstalleur officiel du programme `id`. Ne supprime rien soi-même.
/// ShellExecute (et non CreateProcess) pour que Windows affiche l'UAC si le désinstalleur l'exige.
pub fn uninstall(id: &str, journal_dir: &Path) -> Result<(), String> {
    let (prefix, key_name) = id.split_once('\\').ok_or("Invalid id")?;
    let root = open(prefix).ok_or("Program not found")?;
    let program = read(prefix, &root, key_name).ok_or("Program not found")?;
    let cmd: String = root.open_subkey(key_name).and_then(|k| k.get_value("UninstallString")).map_err(|e| e.to_string())?;
    let (exe, args) = split_command(&cmd).ok_or("Unreadable uninstall command")?;

    let mut entry = journal::Entry::new("uninstall");
    entry.paths = vec![program.name.clone(), cmd.clone()];
    entry.bytes = program.bytes;
    let result = shell_open(&exe, &args);
    entry.error = result.as_ref().err().cloned();
    journal::append(journal_dir, &entry).map_err(|e| format!("Log: {e}"))?;
    result
}

fn shell_open(exe: &str, args: &str) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let w = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let (verb, file, params) = (w("open"), w(exe), w(args));
    let r = unsafe {
        ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(), params.as_ptr(), std::ptr::null(), SW_SHOWNORMAL)
    };
    if r as isize > 32 { Ok(()) } else { Err(format!("Could not start the uninstaller ({exe})")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(s: &str) -> (String, String) {
        split_command(s).unwrap()
    }

    #[test]
    fn ghost_only_when_sure() {
        let dir = std::env::temp_dir().join(format!("kleaner-ghost-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let real = dir.join("unins.exe");
        std::fs::write(&real, b"x").unwrap();
        let gone = dir.join("gone.exe");
        let q = |p: &Path| format!("\"{}\" /S", p.display());
        assert!(!is_ghost(&q(&real), ""), "désinstalleur présent");
        assert!(is_ghost(&q(&gone), ""), "désinstalleur et dossier absents");
        assert!(!is_ghost(&q(&gone), &dir.display().to_string()), "le dossier d'installation existe encore");
        assert!(is_ghost(&q(&gone), &dir.join("nope").display().to_string()));
        assert!(exe_missing(&q(&gone)) && !exe_missing(&q(&real)), "programme absent / présent");
        assert!(!exe_missing("notepad.exe") && !exe_missing(""), "chemin relatif ou vide : on ne conclut pas");
        assert!(!is_ghost("MsiExec.exe /X{ABC}", ""), "chemin relatif : on ne conclut pas");
        assert!(!is_ghost("\"%ProgramFiles%\\x\\u.exe\"", ""), "variable : on ne conclut pas");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Cycle complet dans HKCU (ruche de l'utilisateur) : repérée, sauvegardée, supprimée, restaurée.
    #[test]
    fn ghost_round_trip_in_hkcu() {
        let key_name = format!("KleanerGhostTest-{}", std::process::id());
        let dir = std::env::temp_dir().join(format!("kleaner-ghost-rt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (k, _) = hkcu.create_subkey(format!(r"{UNINSTALL}\{key_name}")).unwrap();
        k.set_value("DisplayName", &"Kleaner ghost test").unwrap();
        k.set_value("UninstallString", &r#""C:\Kleaner-ghost-test-absent\unins000.exe" /S"#).unwrap();
        let id = format!("HKCU\\{key_name}");

        assert!(ghosts().iter().any(|g| g.id == id), "repérée");
        let backup = remove_ghost(&id, &dir.join("backups"), &dir).unwrap();
        assert!(Path::new(&backup).is_file(), "sauvegarde .reg écrite");
        assert!(hkcu.open_subkey(format!(r"{UNINSTALL}\{key_name}")).is_err(), "clé supprimée");
        assert!(remove_ghost(&id, &dir.join("backups"), &dir).is_err(), "plus rien à supprimer");

        restore_ghost(&backup, &dir.join("backups"), &dir).unwrap();
        assert!(hkcu.open_subkey(format!(r"{UNINSTALL}\{key_name}")).is_ok(), "clé restaurée");
        let journal = std::fs::read_to_string(dir.join("journal.log")).unwrap();
        assert!(journal.contains("ghost-remove") && journal.contains("ghost-restore"));

        hkcu.delete_subkey_all(format!(r"{UNINSTALL}\{key_name}")).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn restore_refuses_files_outside_the_backup_folder() {
        let dir = std::env::temp_dir().join(format!("kleaner-ghost-bk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let other = std::env::temp_dir().join(format!("ghost-HKCU-x-{}.reg", std::process::id()));
        std::fs::write(&other, b"x").unwrap();
        let err = restore_ghost(&other.display().to_string(), &dir, &dir).unwrap_err();
        assert!(err.starts_with("Refused"), "{err}");
        std::fs::remove_file(&other).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn versions_compare_numerically() {
        assert!(version_key("136.0.6008.111") > version_key("133.0.5932.39"));
        assert!(version_key("10.2") > version_key("9.99"));
    }

    #[test]
    fn quoted_path_with_args() {
        assert_eq!(
            split(r#""C:\Program Files\App\unins000.exe" /SILENT"#),
            (r"C:\Program Files\App\unins000.exe".into(), "/SILENT".into())
        );
    }

    #[test]
    fn unquoted_path_with_spaces() {
        assert_eq!(
            split(r"C:\Program Files (x86)\Some App\uninstall.exe --remove"),
            (r"C:\Program Files (x86)\Some App\uninstall.exe".into(), "--remove".into())
        );
    }

    #[test]
    fn msiexec_install_becomes_uninstall() {
        assert_eq!(split("MsiExec.exe /I{1234-ABCD}"), ("MsiExec.exe".into(), "/X{1234-ABCD}".into()));
        assert_eq!(split("MsiExec.exe /X{1234-ABCD}"), ("MsiExec.exe".into(), "/X{1234-ABCD}".into()));
    }

    #[test]
    fn empty_is_none() {
        assert!(split_command("  ").is_none());
        assert!(split_command("\"\"").is_none());
    }

    #[test]
    fn clean_path_strips_quotes_and_icon_index() {
        assert_eq!(clean_path(r#""C:\Apps\x\app.exe",0"#), PathBuf::from(r"C:\Apps\x\app.exe"));
        assert_eq!(clean_path(r"C:\Apps\x\"), PathBuf::from(r"C:\Apps\x"));
    }

    #[test]
    fn shared_and_system_folders_never_used() {
        let shared = shared_roots();
        let pf = PathBuf::from(std::env::var_os("ProgramFiles").unwrap());
        let win = PathBuf::from(std::env::var_os("SystemRoot").unwrap());
        assert!(!usable(&pf, &shared), "Program Files entier");
        assert!(!usable(&win.join("System32"), &shared), "dossier Windows");
        assert!(!usable(Path::new("C:\\"), &shared), "racine");
        assert!(!usable(&PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()), &shared));
    }

    #[test]
    fn exe_fallback_only_under_install_roots() {
        let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
        let app = local.join(format!("kleaner-test-app-{}", std::process::id()));
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("a.bin"), vec![0u8; 1234]).unwrap();
        let icon = format!("\"{}\",0", app.join("app.exe").display());
        assert_eq!(location("", &icon, ""), Some(app.clone()));
        assert_eq!(dir_size(&app), 1234);
        // Désinstalleur dans un dossier temporaire : pas sous une racine d'installation.
        let tmp = std::env::temp_dir().join("x").join("unins.exe");
        assert_eq!(location("", "", &tmp.display().to_string()), None);
        std::fs::remove_dir_all(&app).unwrap();
    }

    /// Sur la vraie machine : `cargo test --release programs::tests::real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_sizes_on_this_machine() {
        let all = list();
        let t = std::time::Instant::now();
        let sizes = real_sizes(&all);
        println!("{}/{} programmes avec dossier, {} ms", sizes.len(), all.len(), t.elapsed().as_millis());
        let mut rows: Vec<_> = sizes.iter().map(|r| (r.bytes, all.iter().find(|p| p.id == r.id).unwrap())).collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.0));
        for (real, p) in rows.iter().take(12) {
            let gb = |b: u64| b as f64 / (1u64 << 30) as f64;
            println!("{:>7.2} Go réel  {:>7.2} Go déclaré  {}  [{}]", gb(*real), gb(p.bytes), p.name, p.location);
        }
    }

    #[test]
    fn list_reads_registry() {
        let all = list();
        assert!(!all.is_empty(), "au moins un programme installé");
        assert!(all.windows(2).all(|w| w[0].bytes >= w[1].bytes), "trié par taille");
    }
}

#[cfg(test)]
mod ghost_real_tests {
    /// Sur la vraie machine : `cargo test --release ghost_real_tests -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real() {
        for g in super::ghosts() {
            println!("{} | {} {} | {}", g.id, g.name, g.version, g.command);
        }
    }
}
