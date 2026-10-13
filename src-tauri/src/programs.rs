//! Programmes installés, lus dans le registre (lecture seule).
//! La désinstallation lance uniquement le désinstalleur déclaré par le programme.

use crate::journal;
use serde::Serialize;
use std::path::Path;
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
    })
}

/// Tous les programmes, triés par taille décroissante, sans doublons (même nom + version).
pub fn list() -> Vec<Program> {
    let mut out: Vec<Program> = Vec::new();
    for (prefix, ..) in SOURCES {
        let Some(root) = open(prefix) else { continue };
        for key_name in root.enum_keys().flatten() {
            if let Some(p) = read(prefix, &root, &key_name)
                && !out.iter().any(|o| o.name == p.name && o.version == p.version)
            {
                out.push(p);
            }
        }
    }
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    out
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
    let (prefix, key_name) = id.split_once('\\').ok_or("Identifiant invalide")?;
    let root = open(prefix).ok_or("Programme introuvable")?;
    let program = read(prefix, &root, key_name).ok_or("Programme introuvable")?;
    let cmd: String = root.open_subkey(key_name).and_then(|k| k.get_value("UninstallString")).map_err(|e| e.to_string())?;
    let (exe, args) = split_command(&cmd).ok_or("Commande de désinstallation illisible")?;

    let mut entry = journal::Entry::new("uninstall");
    entry.paths = vec![program.name.clone(), cmd.clone()];
    entry.bytes = program.bytes;
    let result = shell_open(&exe, &args);
    entry.error = result.as_ref().err().cloned();
    journal::append(journal_dir, &entry).map_err(|e| format!("Journal : {e}"))?;
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
    if r as isize > 32 { Ok(()) } else { Err(format!("Impossible de lancer le désinstalleur ({exe})")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(s: &str) -> (String, String) {
        split_command(s).unwrap()
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
    fn list_reads_registry() {
        let all = list();
        assert!(!all.is_empty(), "au moins un programme installé");
        assert!(all.windows(2).all(|w| w[0].bytes >= w[1].bytes), "trié par taille");
    }
}
