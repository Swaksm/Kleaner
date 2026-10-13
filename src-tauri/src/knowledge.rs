//! « Qu'est-ce que c'est ? » : explique un fichier ou dossier et dit si le supprimer casse quelque chose.
//! Ordre : fiches précises → catalogue de nettoyage → programme propriétaire → règles générales → inconnu.
//! Lecture seule. Kleaner ne devine jamais : sans correspondance, la réponse est « inconnu ».

use crate::catalog::{self, Text};
use crate::programs::Program;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const KNOWLEDGE_JSON: &str = include_str!("../knowledge.json");

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// Supprimable sans conséquence (ou recréé automatiquement).
    Safe,
    /// À nettoyer avec l'outil du logiciel ou le Nettoyage de Kleaner.
    Cleanup,
    /// Fait partie d'un programme : désinstaller pour libérer.
    Uninstall,
    /// Le supprimer casse un logiciel ou fait perdre des données.
    Breaks,
    /// Fichiers de l'utilisateur : à lui de décider.
    Personal,
    /// Système : ne jamais toucher.
    System,
    Unknown,
}

impl Verdict {
    /// Supprimer à la main est dangereux : confirmation renforcée.
    pub fn critical(self) -> bool {
        matches!(self, Verdict::Breaks | Verdict::System | Verdict::Uninstall)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    #[serde(rename = "match")]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub generic: bool,
    pub verdict: Verdict,
    #[serde(default)]
    pub cleaner: Option<String>,
    pub what: Text,
    pub consequence: Text,
    pub howto: Text,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Rule,
    Cleaner,
    Program,
    Unknown,
}

#[derive(Serialize, Clone, Debug)]
pub struct Explanation {
    pub path: String,
    pub verdict: Verdict,
    pub source: Source,
    pub rule: Option<String>,
    pub cleaner: Option<String>,
    pub what: Text,
    pub consequence: Text,
    pub howto: Text,
}

pub fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| serde_json::from_str(KNOWLEDGE_JSON).expect("knowledge.json invalide"))
}

/// Le motif est en minuscules ; le chemin garde sa casse (pour les captures).
fn same(a: char, lower: char) -> bool {
    a == lower || a.to_lowercase().eq(std::iter::once(lower))
}

/// Jokers sur un chemin entier, insensibles à la casse :
/// `*` = n'importe quoi sauf `\` (capturé dans `caps`), `?` = un caractère sauf `\`,
/// `**` = n'importe quoi y compris `\`, et `**\` peut ne rien couvrir.
fn glob(p: &[char], s: &[char], caps: &mut Vec<String>) -> bool {
    match p {
        [] => s.is_empty(),
        ['*', '*', '\\', rest @ ..] => {
            glob(rest, s, caps) || (0..s.len()).any(|i| s[i] == '\\' && glob(rest, &s[i + 1..], caps))
        }
        ['*', '*', rest @ ..] => (0..=s.len()).any(|i| glob(rest, &s[i..], caps)),
        ['*', rest @ ..] => {
            let max = s.iter().position(|&c| c == '\\').unwrap_or(s.len());
            for i in (0..=max).rev() {
                caps.push(s[..i].iter().collect());
                if glob(rest, &s[i..], caps) {
                    return true;
                }
                caps.pop();
            }
            false
        }
        ['?', rest @ ..] => !s.is_empty() && s[0] != '\\' && glob(rest, &s[1..], caps),
        [c, rest @ ..] => !s.is_empty() && same(s[0], *c) && glob(rest, &s[1..], caps),
    }
}

fn lower_chars(s: &str) -> Vec<char> {
    s.to_lowercase().trim_end_matches('\\').chars().collect()
}

fn chars(s: &str) -> Vec<char> {
    s.trim_end_matches('\\').chars().collect()
}

/// Le motif désigne-t-il ce chemin ou un dossier qui le contient ? Renvoie les captures des `*`.
fn matches(pattern: &str, path: &[char]) -> Option<Vec<String>> {
    let pat = lower_chars(&catalog::expand_env(pattern)?);
    let ends = path.iter().enumerate().filter(|(_, c)| **c == '\\').map(|(i, _)| i).chain([path.len()]);
    for end in ends {
        let mut caps = Vec::new();
        if glob(&pat, &path[..end], &mut caps) {
            return Some(caps);
        }
    }
    None
}

fn fill(t: &Text, caps: &[String]) -> Text {
    let sub = |s: &str| caps.iter().enumerate().fold(s.to_string(), |acc, (i, c)| acc.replace(&format!("{{{}}}", i + 1), c));
    Text { en: sub(&t.en), fr: sub(&t.fr) }
}

fn from_rule(path: &str, r: &Rule, caps: &[String]) -> Explanation {
    Explanation {
        path: path.to_string(),
        verdict: r.verdict,
        source: Source::Rule,
        rule: Some(r.id.clone()),
        cleaner: r.cleaner.clone(),
        what: fill(&r.what, caps),
        consequence: fill(&r.consequence, caps),
        howto: fill(&r.howto, caps),
    }
}

fn under(path: &[char], dir: &[char]) -> bool {
    path.len() >= dir.len() && path[..dir.len()] == *dir && (path.len() == dir.len() || path[dir.len()] == '\\')
}

/// Contexte calculé une fois par lot : dossiers du catalogue et des programmes.
pub struct Context {
    cleaners: Vec<(Vec<char>, &'static catalog::Cleaner)>,
    programs: Vec<(Vec<char>, Program)>,
}

impl Context {
    pub fn new(programs: Vec<Program>) -> Self {
        let cleaners = catalog::all()
            .iter()
            .filter(|c| c.mode == catalog::Mode::Delete)
            .flat_map(|c| c.paths.iter().flat_map(|p| catalog::resolve(p)).map(move |p| (lower_chars(&p.display().to_string()), c)))
            .collect();
        let mut programs: Vec<_> =
            programs.into_iter().filter(|p| !p.location.is_empty()).map(|p| (lower_chars(&p.location), p)).collect();
        // Le dossier le plus précis gagne.
        programs.sort_by_key(|(dir, _)| std::cmp::Reverse(dir.len()));
        Self { cleaners, programs }
    }

    pub fn explain(&self, path: &str) -> Explanation {
        let lower = lower_chars(path);
        let original = chars(path);
        let first = |generic: bool| {
            rules().iter().filter(|r| r.generic == generic).find_map(|r| {
                r.patterns.iter().find_map(|p| matches(p, &original)).map(|caps| from_rule(path, r, &caps))
            })
        };
        if let Some(e) = first(false) {
            return e;
        }
        if let Some((_, c)) = self.cleaners.iter().find(|(dir, _)| under(&lower, dir)) {
            return Explanation {
                path: path.to_string(),
                verdict: Verdict::Cleanup,
                source: Source::Cleaner,
                rule: None,
                cleaner: Some(c.id.clone()),
                what: c.what.clone(),
                consequence: c.effect.clone(),
                howto: Text {
                    en: format!("Kleaner cleans it safely: Cleanup tab → {}.", c.label.en),
                    fr: format!("Kleaner le nettoie sans risque : onglet Nettoyage → {}.", c.label.fr),
                },
            };
        }
        if let Some((_, p)) = self.programs.iter().find(|(dir, _)| under(&lower, dir)) {
            let who = if p.publisher.is_empty() { p.name.clone() } else { format!("{} ({})", p.name, p.publisher) };
            return Explanation {
                path: path.to_string(),
                verdict: Verdict::Uninstall,
                source: Source::Program,
                rule: None,
                cleaner: None,
                what: Text { en: format!("Part of the program {who}."), fr: format!("Fait partie du programme {who}.") },
                consequence: Text {
                    en: format!("Deleting files by hand breaks {} and leaves it half-installed.", p.name),
                    fr: format!("Supprimer à la main casse {} et le laisse à moitié installé.", p.name),
                },
                howto: Text {
                    en: format!("Uninstall {} from the Programs tab, or remove content from inside the app (e.g. games in its launcher).", p.name),
                    fr: format!("Désinstalle {} depuis l’onglet Programmes, ou retire le contenu depuis l’appli (ex. les jeux dans leur launcher).", p.name),
                },
            };
        }
        if let Some(e) = first(true) {
            return e;
        }
        Explanation {
            path: path.to_string(),
            verdict: Verdict::Unknown,
            source: Source::Unknown,
            rule: None,
            cleaner: None,
            what: Text { en: "Kleaner doesn’t know this item.".into(), fr: "Kleaner ne connaît pas cet élément.".into() },
            consequence: Text {
                en: "Unknown: don’t delete it unless you know what it is.".into(),
                fr: "Inconnu : ne le supprime pas sans savoir ce que c’est.".into(),
            },
            howto: Text {
                en: "Open it in Explorer to see what’s inside, search the folder name online, or use « Report for an AI ».".into(),
                fr: "Ouvre-le dans l’Explorateur pour voir le contenu, cherche le nom du dossier en ligne, ou utilise « Rapport pour une IA ».".into(),
            },
        }
    }
}

/// Explique un lot de chemins (ex. les plus gros fichiers et dossiers de l'analyse).
pub fn explain_all(paths: &[String]) -> Vec<Explanation> {
    let ctx = Context::new(crate::programs::list());
    paths.iter().map(|p| ctx.explain(p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context { cleaners: Vec::new(), programs: Vec::new() }
    }

    fn local(rest: &str) -> String {
        format!("{}\\{rest}", std::env::var("LOCALAPPDATA").unwrap())
    }

    fn profile(rest: &str) -> String {
        format!("{}\\{rest}", std::env::var("USERPROFILE").unwrap())
    }

    fn g(p: &str, s: &str) -> bool {
        glob(&lower_chars(p), &chars(s), &mut Vec::new())
    }

    #[test]
    fn glob_semantics() {
        assert!(g(r"c:\a\*\c", r"c:\a\b\c"));
        assert!(!g(r"c:\a\*\c", r"c:\a\b\x\c"));
        assert!(g(r"c:\a\**\c", r"c:\a\b\x\c"));
        assert!(g(r"c:\a\**\c", r"c:\a\c"), "**\\ peut ne rien couvrir");
        assert!(g(r"**\node_modules", r"d:\proj\web\node_modules"));
        assert!(g(r"**\*.iso", r"c:\Users\me\Downloads\ubuntu.ISO"));
        assert!(g(r"?:\$recycle.bin", r"D:\$Recycle.Bin"));
        assert!(!g(r"?:\$recycle.bin", r"D:\x\$Recycle.Bin"));
    }

    #[test]
    fn docker_vhdx_breaks_with_howto() {
        let e = ctx().explain(&local(r"Docker\wsl\disk\docker_data.vhdx"));
        assert_eq!((e.verdict, e.rule.as_deref()), (Verdict::Breaks, Some("docker-vhdx")));
        assert!(e.howto.en.contains("docker system prune"));
    }

    #[test]
    fn inside_a_known_folder_inherits() {
        let e = ctx().explain(&profile(r"Projects\web\node_modules\react\index.js"));
        assert_eq!(e.rule.as_deref(), Some("node-modules"));
    }

    #[test]
    fn captures_fill_names_with_original_case() {
        let e = ctx().explain(r"D:\SteamLibrary\steamapps\common\Hollow Knight\data.bin");
        assert_eq!(e.rule.as_deref(), Some("steam-game"));
        assert!(e.what.en.contains("Hollow Knight"), "{}", e.what.en);
    }

    #[test]
    fn specific_beats_generic_and_order_matters() {
        let saves = ctx().explain(&format!(r"{}\.minecraft\saves\World1", std::env::var("APPDATA").unwrap()));
        assert_eq!(saves.rule.as_deref(), Some("minecraft-saves"));
        let vdi = ctx().explain(&profile(r"VirtualBox VMs\ubuntu\ubuntu.vdi"));
        assert_eq!(vdi.rule.as_deref(), Some("virtualbox"));
        let other_vm = ctx().explain(r"E:\vms\test.vdi");
        assert_eq!(other_vm.rule.as_deref(), Some("vm-disk"));
    }

    #[test]
    fn system_files() {
        let drive = std::env::var("SystemDrive").unwrap();
        assert_eq!(ctx().explain(&format!(r"{drive}\pagefile.sys")).verdict, Verdict::System);
        let win = std::env::var("SystemRoot").unwrap();
        assert_eq!(ctx().explain(&format!(r"{win}\System32\ntoskrnl.exe")).verdict, Verdict::System);
        assert_eq!(ctx().explain(&format!(r"{win}\Installer\2fab2f.msi")).rule.as_deref(), Some("windows-installer"));
    }

    #[test]
    fn owner_program_detected_before_generic() {
        let p = Program {
            id: "x".into(),
            name: "Diablo II Resurrected".into(),
            publisher: "Blizzard".into(),
            version: String::new(),
            bytes: 0,
            install_date: String::new(),
            location: r"C:\Program Files (x86)\Diablo II Resurrected".into(),
        };
        let c = Context::new(vec![p]);
        let e = c.explain(r"C:\Program Files (x86)\Diablo II Resurrected\Data\data\data.016");
        assert_eq!((e.verdict, e.source), (Verdict::Uninstall, Source::Program));
        assert!(e.what.en.contains("Diablo II Resurrected"));
    }

    #[test]
    fn unknown_stays_unknown() {
        let e = ctx().explain(r"Z:\something\weird.xyz");
        assert_eq!(e.verdict, Verdict::Unknown);
    }

    #[test]
    fn knowledge_is_valid() {
        let all = rules();
        let mut ids: Vec<&str> = all.iter().map(|r| r.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len(), "ids en double");
        let mut seen_generic = false;
        for r in all {
            assert!(!r.patterns.is_empty(), "{}", r.id);
            // Les règles générales sont regroupées après les précises (lisibilité du fichier).
            assert!(!(seen_generic && !r.generic), "{} : règle précise après une générale", r.id);
            seen_generic |= r.generic;
            for p in &r.patterns {
                assert!(p.starts_with('%') || p.starts_with("**\\") || p.starts_with("?:\\"), "{} : {p}", r.id);
                assert!(!p.chars().any(char::is_control), "{} : {p:?}", r.id);
                assert!(catalog::expand_env(p).is_some() || p.contains("%PUBLIC%"), "{} : variable inconnue dans {p}", r.id);
            }
            for t in [&r.what, &r.consequence, &r.howto] {
                for s in [&t.en, &t.fr] {
                    assert!(!s.trim().is_empty(), "{} : texte vide", r.id);
                    assert!(!s.chars().any(char::is_control), "{} : caractère de contrôle", r.id);
                    assert!(s.chars().count() <= 400, "{} : texte trop long", r.id);
                }
            }
            if let Some(c) = &r.cleaner {
                assert!(catalog::get(c).is_some(), "{} : nettoyage inconnu {c}", r.id);
            }
            // Jamais « supprimable » pour une racine large.
            if r.verdict == Verdict::Safe {
                for p in &r.patterns {
                    let segments = p.split('\\').filter(|s| !s.is_empty()).count();
                    assert!(segments >= 2, "{} : {p} trop large pour « safe »", r.id);
                }
            }
        }
    }

    /// Sur la vraie machine : `cargo test --release knowledge::tests::real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_top_files() {
        let progress = crate::scan::Progress::default();
        let r = crate::scan::scan(&[std::path::PathBuf::from("C:\\")], &std::sync::atomic::AtomicBool::new(false), &progress);
        let paths: Vec<String> = r.top_files.iter().chain(&r.top_dirs).take(120).map(|i| i.path.clone()).collect();
        for e in explain_all(&paths) {
            println!("{:<9?} {:<22} {}", e.verdict, e.rule.as_deref().or(e.cleaner.as_deref()).unwrap_or("-"), e.path);
        }
    }
}
