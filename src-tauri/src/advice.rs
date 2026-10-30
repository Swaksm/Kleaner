//! Conseils sans IA, sur des règles simples et vérifiables. Lecture seule : la Corbeille ou Steam font le reste.
//! - installeurs (.exe, .msi, .msix…) de Téléchargements vieux de plus de 30 jours : on peut les retélécharger ;
//! - archives (.zip, .7z, .rar) de Téléchargements à côté d'un dossier du même nom : déjà extraites ;
//! - jeux Steam pas lancés depuis 6 mois : Steam les désinstalle et les réinstalle sur demande.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DAY: u64 = 86_400;
const OLD_INSTALLER: u64 = 30 * DAY;
const OLD_GAME: u64 = 182 * DAY;
const INSTALLERS: [&str; 6] = ["exe", "msi", "msix", "msixbundle", "appx", "appxbundle"];
const ARCHIVES: [&str; 3] = ["zip", "7z", "rar"];

#[derive(Serialize, Clone, Debug)]
pub struct Advice {
    /// installer, archive ou game.
    pub kind: &'static str,
    pub path: String,
    pub name: String,
    pub size: u64,
    /// Secondes Unix : date du fichier, ou dernier lancement du jeu.
    pub modified: u64,
    /// Jeu Steam : son identifiant, pour `steam://uninstall/<id>`.
    pub app_id: Option<u32>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Installeurs et archives déjà extraites, au premier niveau de `dir`. Liens jamais suivis.
pub fn downloads_in(dir: &Path, now: u64) -> Vec<Advice> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out = Vec::new();
    for e in entries.flatten() {
        let Ok(m) = e.path().symlink_metadata() else { continue };
        if !m.is_file() {
            continue;
        }
        let path = e.path();
        let ext = path.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
        let modified = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
        let kind = if INSTALLERS.contains(&ext.as_str()) && modified > 0 && now.saturating_sub(modified) > OLD_INSTALLER {
            "installer"
        } else if ARCHIVES.contains(&ext.as_str()) && path.file_stem().is_some_and(|s| dir.join(s).is_dir()) {
            "archive"
        } else {
            continue;
        };
        let name = e.file_name().to_string_lossy().into_owned();
        out.push(Advice { kind, path: path.display().to_string(), name, size: m.len(), modified, app_id: None });
    }
    out
}

/// Paires `"clé" "valeur"` d'un fichier VDF de Steam (une par ligne), `\\` remis en `\`.
fn vdf_pairs(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let parts: Vec<&str> = l.split('"').collect();
            // « "a" "b" » donne ["", a, ws, b, ""].
            (parts.len() == 5).then(|| (parts[1].to_lowercase(), parts[3].replace(r"\\", r"\")))
        })
        .collect()
}

fn steam_root() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    let path: String = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam").ok()?.get_value("SteamPath").ok()?;
    Some(PathBuf::from(path.replace('/', r"\")))
}

/// Jeux d'une bibliothèque Steam (`<lib>\steamapps`) pas lancés depuis 6 mois. Jamais lancé (0) : ignoré, Steam ne le sait pas.
pub fn games_in(steamapps: &Path, now: u64) -> Vec<Advice> {
    let Ok(entries) = std::fs::read_dir(steamapps) else { return Vec::new() };
    let mut out = Vec::new();
    for e in entries.flatten() {
        let file = e.file_name().to_string_lossy().to_lowercase();
        if !(file.starts_with("appmanifest_") && file.ends_with(".acf")) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
        let pairs = vdf_pairs(&text);
        let get = |k: &str| pairs.iter().find(|(key, _)| key == k).map(|(_, v)| v.as_str());
        let (Some(id), Some(name), Some(dir)) = (get("appid").and_then(|v| v.parse().ok()), get("name"), get("installdir")) else {
            continue;
        };
        let last = get("lastplayed").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        if last == 0 || now.saturating_sub(last) < OLD_GAME {
            continue;
        }
        let size = get("sizeondisk").and_then(|v| v.parse().ok()).unwrap_or(0);
        let path = steamapps.join("common").join(dir).display().to_string();
        out.push(Advice { kind: "game", path, name: name.to_string(), size, modified: last, app_id: Some(id) });
    }
    out
}

pub fn find() -> Vec<Advice> {
    let now = now();
    // shortcut: Téléchargements déplacé ailleurs (Propriétés → Emplacement) n'est pas suivi, comme dans mover.rs.
    let mut out = std::env::var_os("USERPROFILE").map(|h| downloads_in(&PathBuf::from(h).join("Downloads"), now)).unwrap_or_default();
    if let Some(root) = steam_root() {
        let mut libs = vec![root.clone()];
        if let Ok(text) = std::fs::read_to_string(root.join(r"steamapps\libraryfolders.vdf")) {
            libs.extend(vdf_pairs(&text).into_iter().filter(|(k, _)| k == "path").map(|(_, v)| PathBuf::from(v)));
        }
        libs.sort_by_key(|p| p.display().to_string().to_lowercase());
        libs.dedup_by_key(|p| p.display().to_string().to_lowercase());
        for lib in libs {
            out.extend(games_in(&lib.join("steamapps"), now));
        }
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.size));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sandbox(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kleaner-advice-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn old_installers_and_extracted_archives_only() {
        let d = sandbox("dl");
        for f in ["setup.exe", "tool.msi", "photos.zip", "lonely.zip", "notes.txt"] {
            fs::write(d.join(f), b"x").unwrap();
        }
        fs::create_dir(d.join("photos")).unwrap();
        let later = now() + 31 * DAY;
        let mut found: Vec<(&str, String)> = downloads_in(&d, later).into_iter().map(|a| (a.kind, a.name)).collect();
        found.sort();
        assert_eq!(found, [("archive", "photos.zip".into()), ("installer", "setup.exe".into()), ("installer", "tool.msi".into())]);
        assert!(downloads_in(&d, now()).iter().all(|a| a.kind == "archive"), "installeur récent : gardé");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn steam_games_not_played_for_six_months() {
        let d = sandbox("steam");
        let manifest = |id: u32, last: u64| {
            format!("\"AppState\"\n{{\n\t\"appid\"\t\t\"{id}\"\n\t\"name\"\t\t\"Game {id}\"\n\t\"installdir\"\t\t\"G{id}\"\n\t\"LastPlayed\"\t\t\"{last}\"\n\t\"SizeOnDisk\"\t\t\"5000\"\n}}\n")
        };
        let t = now();
        fs::write(d.join("appmanifest_1.acf"), manifest(1, t - 200 * DAY)).unwrap();
        fs::write(d.join("appmanifest_2.acf"), manifest(2, t - DAY)).unwrap();
        fs::write(d.join("appmanifest_3.acf"), manifest(3, 0)).unwrap();
        let games = games_in(&d, t);
        assert_eq!(games.len(), 1);
        assert_eq!((games[0].app_id, games[0].size, games[0].name.as_str()), (Some(1), 5000, "Game 1"));
        assert!(games[0].path.ends_with(r"common\G1"));
        fs::remove_dir_all(&d).unwrap();
    }

    /// Sur la vraie machine : `cargo test --release advice::tests::real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real() {
        for a in find() {
            println!("{:9} {:>12} {} {:?}", a.kind, a.size, a.path, a.app_id);
        }
    }

    #[test]
    fn vdf_paths_are_unescaped() {
        let pairs = vdf_pairs("\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\"0\"\n{");
        assert_eq!(pairs, [("path".to_string(), r"D:\SteamLibrary".to_string())]);
    }
}
