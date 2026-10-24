//! Journal des actions : une ligne JSON par action dans `%LOCALAPPDATA%\Kleaner\journal.log`.
//! Lisible dans l'appli et dans n'importe quel éditeur texte.

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const FILE: &str = "journal.log";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Entry {
    /// Secondes depuis l'epoch Unix.
    pub ts: u64,
    /// Ex. "trash", "clean:browser-chrome-cache", "uninstall".
    pub action: String,
    pub paths: Vec<String>,
    pub files: u64,
    pub bytes: u64,
    /// Fichiers ignorés (verrouillés, refusés par la garde…).
    pub skipped: u64,
    pub error: Option<String>,
    /// Sortie d'une commande Windows (DISM, powercfg…).
    pub output: Option<String>,
}

impl Entry {
    pub fn new(action: impl Into<String>) -> Self {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        Self { ts, action: action.into(), ..Default::default() }
    }
}

/// Dossier de données de Kleaner : seul endroit où l'appli écrit.
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Kleaner")
}

pub fn append(dir: &Path, entry: &Entry) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let mut f = OpenOptions::new().create(true).append(true).open(dir.join(FILE))?;
    let line = serde_json::to_string(entry).map_err(io::Error::other)?;
    writeln!(f, "{line}")
}

/// Les `n` dernières entrées, la plus récente en premier. Les lignes illisibles sont ignorées.
pub fn read_last(dir: &Path, n: usize) -> io::Result<Vec<Entry>> {
    let text = match fs::read_to_string(dir.join(FILE)) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    Ok(text.lines().rev().filter_map(|l| serde_json::from_str(l).ok()).take(n).collect())
}

/// Espace libéré par Kleaner depuis le début du journal : (octets, date de la première action, en secondes).
/// Seuls les nettoyages (`clean:…`, Corbeille vidée comprise) et « Libérer de l'espace » OneDrive comptent :
/// une mise à la Corbeille ne libère rien tant qu'elle n'est pas vidée, et la compter ferait un doublon.
pub fn freed_total(dir: &Path) -> io::Result<(u64, u64)> {
    let entries = read_last(dir, usize::MAX)?;
    let freeing = entries.iter().filter(|e| e.action.starts_with("clean:") || e.action == "cloud-free-up");
    let bytes = freeing.clone().map(|e| e.bytes).sum();
    let first = freeing.map(|e| e.ts).min().unwrap_or(0);
    Ok((bytes, first))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kleaner-journal-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn append_then_read_newest_first() {
        let d = temp("order");
        for i in 0..3 {
            let mut e = Entry::new(format!("a{i}"));
            e.bytes = i;
            append(&d, &e).unwrap();
        }
        let got = read_last(&d, 2).unwrap();
        assert_eq!(got.iter().map(|e| e.action.as_str()).collect::<Vec<_>>(), ["a2", "a1"]);
        assert_eq!(got[0].bytes, 2);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn missing_journal_is_empty() {
        assert!(read_last(&temp("missing"), 10).unwrap().is_empty());
    }

    #[test]
    fn corrupted_line_skipped() {
        let d = temp("corrupt");
        append(&d, &Entry::new("ok1")).unwrap();
        fs::write(d.join(FILE), format!("{}not json\n", fs::read_to_string(d.join(FILE)).unwrap())).unwrap();
        append(&d, &Entry::new("ok2")).unwrap();
        let got = read_last(&d, 10).unwrap();
        assert_eq!(got.iter().map(|e| e.action.as_str()).collect::<Vec<_>>(), ["ok2", "ok1"]);
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn data_dir_under_localappdata() {
        assert!(data_dir().ends_with("Kleaner"));
    }

    #[test]
    fn freed_total_counts_cleanups_only() {
        let dir = std::env::temp_dir().join(format!("kleaner-freed-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for (action, bytes, ts) in [("clean:temp", 100, 50), ("trash", 1000, 10), ("cloud-free-up", 20, 70), ("clean:recycle-bin", 5, 60)] {
            let e = Entry { ts, action: action.into(), bytes, ..Default::default() };
            append(&dir, &e).unwrap();
        }
        assert_eq!(freed_total(&dir).unwrap(), (125, 50), "la Corbeille ne compte pas, première date = premier nettoyage");
        assert_eq!(freed_total(&dir.join("vide")).unwrap(), (0, 0));
        fs::remove_dir_all(&dir).unwrap();
    }
}
