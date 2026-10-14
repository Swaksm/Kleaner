//! Raccourcis cassés : fichiers `.lnk` du Bureau, du menu Démarrer et de la barre des tâches dont la cible
//! n'existe plus. Lecture seule ; le nettoyage passe par la Corbeille (restaurable depuis le Journal).
//! Un raccourci n'est « cassé » que si sa cible est sur un disque présent et local : cible réseau,
//! disque débranché, application du Store ou raccourci sans chemin (installateur MSI) sont ignorés.

use crate::scan::{self, DriveKind};
use crate::system::ps_json;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Clone, Debug)]
pub struct BrokenShortcut {
    pub path: String,
    pub target: String,
    /// desktop, startmenu ou taskbar.
    pub place: &'static str,
    /// Secondes Unix.
    pub modified: u64,
    /// Dossier commun à tous les utilisateurs : droits admin requis.
    pub machine: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase", default)]
struct Row {
    path: String,
    target: String,
}

fn env(k: &str) -> Option<PathBuf> {
    std::env::var_os(k).map(PathBuf::from)
}

/// (dossier, emplacement, commun à tous)
fn folders() -> Vec<(PathBuf, &'static str, bool)> {
    let mut v = Vec::new();
    let mut add = |p: Option<PathBuf>, place, machine| {
        if let Some(p) = p.filter(|p| p.is_dir()) {
            v.push((p, place, machine));
        }
    };
    add(env("USERPROFILE").map(|p| p.join("Desktop")), "desktop", false);
    for k in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        add(env(k).map(|p| p.join("Desktop")), "desktop", false);
    }
    add(env("PUBLIC").map(|p| p.join("Desktop")), "desktop", true);
    add(env("APPDATA").map(|p| p.join(r"Microsoft\Windows\Start Menu")), "startmenu", false);
    add(env("ProgramData").map(|p| p.join(r"Microsoft\Windows\Start Menu")), "startmenu", true);
    add(env("APPDATA").map(|p| p.join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar")), "taskbar", false);
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v.dedup_by(|a, b| a.0 == b.0);
    v
}

/// `roots` : racines des disques locaux présents, en minuscules (« c:\ »).
pub fn is_broken(target: &str, roots: &[String]) -> bool {
    let t = target.trim();
    let b = t.as_bytes();
    if b.len() < 4 || b[1] != b':' || b[2] != b'\\' || !b[0].is_ascii_alphabetic() {
        return false; // vide, UNC, protocole, shell:…
    }
    let root = format!("{}:\\", (b[0] as char).to_ascii_lowercase());
    if !roots.contains(&root) || t.to_lowercase().contains(r"\windowsapps\") {
        return false;
    }
    matches!(std::fs::symlink_metadata(t), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
}

pub fn find() -> Result<Vec<BrokenShortcut>, String> {
    let roots: Vec<String> = scan::drives()
        .into_iter()
        .filter(|d| matches!(d.kind, DriveKind::Fixed | DriveKind::Removable))
        .map(|d| d.root.to_lowercase())
        .collect();
    let folders = folders();
    if folders.is_empty() {
        return Ok(Vec::new());
    }
    let list = folders.iter().map(|(p, ..)| format!("'{}'", p.display().to_string().replace('\'', "''"))).collect::<Vec<_>>().join(",");
    let rows: Vec<Row> = ps_json(&format!(
        r#"$sh = New-Object -ComObject WScript.Shell;
        ConvertTo-Json -Compress -InputObject @(Get-ChildItem -LiteralPath {list} -Recurse -Filter *.lnk -Force -ErrorAction SilentlyContinue |
          ForEach-Object {{ try {{ [pscustomobject]@{{ Path = $_.FullName; Target = $sh.CreateShortcut($_.FullName).TargetPath }} }} catch {{ }} }})"#
    ))?;
    let mut out: Vec<BrokenShortcut> = rows
        .into_iter()
        // « Tombstones » : Windows y garde la trace des épingles retirées de la barre des tâches.
        .filter(|r| !r.path.to_lowercase().contains(r"\tombstones\") && is_broken(&r.target, &roots))
        .filter_map(|r| {
            let lower = r.path.to_lowercase();
            let (_, place, machine) = folders.iter().filter(|(p, ..)| lower.starts_with(&p.display().to_string().to_lowercase())).max_by_key(|(p, ..)| p.as_os_str().len())?;
            let modified = std::fs::metadata(&r.path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            Some(BrokenShortcut { path: r.path, target: r.target, place, modified, machine: *machine })
        })
        .collect();
    out.sort_by_key(|a| a.path.to_lowercase());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots() -> Vec<String> {
        vec!["c:\\".into(), "d:\\".into()]
    }

    #[test]
    fn missing_target_on_a_present_disk_is_broken() {
        let dir = std::env::temp_dir().join(format!("kleaner-lnk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let real = dir.join("here.exe");
        std::fs::write(&real, b"x").unwrap();
        let drive = real.display().to_string().chars().next().unwrap().to_ascii_lowercase();
        let roots = vec![format!("{drive}:\\")];
        assert!(!is_broken(&real.display().to_string(), &roots));
        assert!(is_broken(&dir.join("gone.exe").display().to_string(), &roots));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_cases_are_never_called_broken() {
        let r = roots();
        assert!(!is_broken("", &r), "MSI : pas de chemin");
        assert!(!is_broken(r"\\server\share\app.exe", &r), "réseau");
        assert!(!is_broken(r"Q:\unplugged\app.exe", &r), "disque absent");
        assert!(!is_broken(r"C:\Program Files\WindowsApps\x\app.exe", &r), "Store");
        assert!(!is_broken("shell:AppsFolder\\x", &r));
    }
}

#[cfg(test)]
mod real_tests {
    /// Sur la vraie machine : `cargo test --release shortcuts::real_tests -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real() {
        for s in super::find().unwrap() {
            println!("{} {} -> {} (machine={})", s.place, s.path, s.target, s.machine);
        }
    }
}
