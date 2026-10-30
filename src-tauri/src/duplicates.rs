//! Fichiers en double : même taille, puis même échantillon (début + fin), puis même contenu complet.
//! Lecture seule. Ne compare que les fichiers de l'utilisateur d'au moins 1 Mo (voir `scan`).

use crate::scan::{DupCandidate, Item};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

const SAMPLE: u64 = 64 * 1024;
const MAX_GROUPS: usize = 500;

#[derive(Serialize, Clone, Debug)]
pub struct DupGroup {
    pub size: u64,
    /// Place récupérable en gardant une seule copie.
    pub wasted: u64,
    pub files: Vec<Item>,
}

fn group_by<K: std::hash::Hash + Eq + Send>(
    files: Vec<DupCandidate>,
    key: impl Fn(&DupCandidate) -> Option<K> + Sync,
) -> Vec<Vec<DupCandidate>> {
    let keyed: Vec<(K, DupCandidate)> = files.into_par_iter().filter_map(|f| key(&f).map(|k| (k, f))).collect();
    let mut map: HashMap<K, Vec<DupCandidate>> = HashMap::new();
    for (k, f) in keyed {
        map.entry(k).or_default().push(f);
    }
    map.into_values().filter(|g| g.len() > 1).collect()
}

/// Début et fin du fichier : écarte vite les fichiers de même taille mais différents.
fn sample_hash(path: &Path, size: u64) -> io::Result<[u8; 32]> {
    let mut f = File::open(path)?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; SAMPLE.min(size) as usize];
    f.read_exact(&mut buf)?;
    h.update(&buf);
    if size > SAMPLE {
        f.seek(SeekFrom::Start(size - SAMPLE))?;
        f.read_exact(&mut buf)?;
        h.update(&buf);
    }
    Ok(*h.finalize().as_bytes())
}

fn full_hash(path: &Path, cancel: &AtomicBool, done: &AtomicU64) -> io::Result<[u8; 32]> {
    let mut f = File::open(path)?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        if cancel.load(Relaxed) {
            return Err(io::Error::other("cancelled"));
        }
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        done.fetch_add(n as u64, Relaxed);
    }
    Ok(*h.finalize().as_bytes())
}

/// Sans les fichiers d'un projet de code (un dossier parent contient `.git`, `package.json`…) : ses copies
/// (assets, build, dist) servent chacune à leur place et se recréent ou cassent le projet. On ne remonte pas
/// au-delà de `stop` (dossier personnel, Bureau, Documents…) ni jusqu'à la racine du disque.
pub fn outside_projects(candidates: Vec<DupCandidate>, stop: &[PathBuf]) -> Vec<DupCandidate> {
    let stop: Vec<String> = stop.iter().map(|s| s.to_string_lossy().to_lowercase()).collect();
    let mut is_project: HashMap<PathBuf, bool> = HashMap::new();
    candidates
        .into_iter()
        .filter(|c| {
            !c.path
                .ancestors()
                .skip(1)
                .take_while(|a| a.parent().is_some() && !stop.contains(&a.to_string_lossy().to_lowercase()))
                .any(|a| {
                    *is_project.entry(a.to_path_buf()).or_insert_with(|| {
                        std::fs::read_dir(a).is_ok_and(|rd| rd.flatten().any(|e| crate::mover::project_marker(&e.file_name())))
                    })
                })
        })
        .collect()
}

/// Groupes de fichiers identiques, du plus de place récupérable au moins.
/// `on_progress(octets lus, octets à lire)` est appelé pendant la comparaison complète.
pub fn find(candidates: Vec<DupCandidate>, cancel: &AtomicBool, on_progress: &(dyn Fn(u64, u64) + Sync)) -> Vec<DupGroup> {
    let by_size = group_by(candidates, |f| Some(f.size));
    let by_sample: Vec<Vec<DupCandidate>> = by_size
        .into_iter()
        .flat_map(|g| group_by(g, |f| sample_hash(&f.path, f.size).ok().map(|h| (f.size, h))))
        .collect();
    let total: u64 = by_sample.iter().flatten().map(|f| f.size).sum();
    let done = AtomicU64::new(0);
    let mut groups: Vec<DupGroup> = Vec::new();
    for g in by_sample {
        if cancel.load(Relaxed) {
            break;
        }
        for same in group_by(g, |f| full_hash(&f.path, cancel, &done).ok()) {
            let size = same[0].size;
            let mut files: Vec<Item> = same
                .into_iter()
                .map(|f| Item { size: f.size, path: f.path.display().to_string(), modified: f.modified })
                .collect();
            files.sort_by_key(|i| i.modified);
            groups.push(DupGroup { size, wasted: size * (files.len() as u64 - 1), files });
        }
        on_progress(done.load(Relaxed), total);
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.wasted));
    groups.truncate(MAX_GROUPS);
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kleaner-dups-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn cand(p: &Path) -> DupCandidate {
        DupCandidate { path: p.to_path_buf(), size: fs::metadata(p).unwrap().len(), modified: 0 }
    }

    #[test]
    fn identical_grouped_and_same_size_different_content_not() {
        let d = temp("basic");
        let big: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let mut other = big.clone();
        other[150_000] ^= 1; // même taille, même début et fin, contenu différent au milieu
        for (name, data) in [("a.bin", &big), ("b.bin", &big), ("c.bin", &other), ("d.bin", &big[..1000].to_vec())] {
            fs::write(d.join(name), data).unwrap();
        }
        let files: Vec<DupCandidate> = ["a.bin", "b.bin", "c.bin", "d.bin"].iter().map(|n| cand(&d.join(n))).collect();
        let groups = find(files, &AtomicBool::new(false), &|_, _| {});
        assert_eq!(groups.len(), 1, "{groups:?}");
        assert_eq!(groups[0].files.len(), 2);
        assert_eq!(groups[0].wasted, 300_000);
        assert!(!groups[0].files.iter().any(|f| f.path.ends_with("c.bin")));
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn files_of_code_projects_left_out() {
        let d = temp("projects");
        for (p, marker) in [("game/assets/sky.png", Some("game/.git")), ("app/dist/x/sky.png", Some("app/package.json")), ("photos/sky.png", None)] {
            fs::create_dir_all(d.join(p).parent().unwrap()).unwrap();
            fs::write(d.join(p), b"x").unwrap();
            if let Some(m) = marker {
                fs::create_dir_all(d.join(m).parent().unwrap()).unwrap();
                fs::write(d.join(m), b"").unwrap();
            }
        }
        // Un `.git` dans le dossier d'arrêt (dossier personnel) n'écarte rien.
        fs::write(d.join(".git"), b"").unwrap();
        let files = ["game/assets/sky.png", "app/dist/x/sky.png", "photos/sky.png"].iter().map(|p| cand(&d.join(p))).collect();
        let kept = outside_projects(files, std::slice::from_ref(&d));
        assert_eq!(kept.iter().map(|c| c.path.clone()).collect::<Vec<_>>(), vec![d.join("photos/sky.png")]);
        fs::remove_dir_all(&d).unwrap();
    }
}

#[cfg(test)]
mod real {
    /// Sur la vraie machine : `cargo test --release duplicates::real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_machine() {
        use std::sync::atomic::AtomicBool;
        let t = std::time::Instant::now();
        let (r, cands) = crate::scan::scan_full(&crate::scan::fixed_drives(), &AtomicBool::new(false), &crate::scan::Progress::default());
        println!("scan {} ms, {} candidats doublons, {} dossiers projets, {} noeuds carte", t.elapsed().as_millis(), cands.len(), r.artifacts.len(), r.tree.len());
        let t = std::time::Instant::now();
        let groups = super::find(cands, &AtomicBool::new(false), &|_, _| {});
        let wasted: u64 = groups.iter().map(|g| g.wasted).sum();
        println!("doublons {} ms, {} groupes, {:.2} Go récupérables", t.elapsed().as_millis(), groups.len(), wasted as f64 / (1u64 << 30) as f64);
        for g in groups.iter().take(6) {
            println!("  {:.1} Mo x{}  {}", g.size as f64 / 1048576.0, g.files.len(), g.files[0].path);
        }
        let art: u64 = r.artifacts.iter().map(|a| a.size).sum();
        println!("projets : {:.2} Go", art as f64 / (1u64 << 30) as f64);
        for a in r.artifacts.iter().take(8) {
            println!("  {:<7} {:>7.0} Mo  {}", a.kind, a.size as f64 / 1048576.0, a.path);
        }
    }
}
