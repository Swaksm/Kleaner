//! Mise à jour en un clic (version installée seulement) : télécharge `kleaner-setup.exe` de la version choisie,
//! vérifie son empreinte SHA-256 publiée à côté, puis lance l'installateur sans assistant ; l'appelant ferme Kleaner.
//! L'installateur Tauri : `/P` barre de progression seule (et ferme Kleaner s'il tourne encore), `/UPDATE` garde
//! l'installation et les raccourcis, `/R` relance Kleaner à la fin. Seule la demande admin de Windows reste.
//! Uniquement sur clic. L'adresse est construite ici à partir d'un numéro de version vérifié, jamais reçue de l'interface.

use crate::journal;
use crate::system::ps_run;
use std::path::{Path, PathBuf};

const RELEASES: &str = "https://github.com/Swaksm/Kleaner/releases/download";

/// Version installée par l'installateur (son désinstalleur est à côté de l'exe), et non la version portable.
pub fn is_installed() -> bool {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("uninstall.exe"))).is_some_and(|u| u.is_file())
}

/// « 1.2.3 » ou « 1.2.3-rc4 » (avec ou sans « v ») : rien d'autre ne peut entrer dans l'adresse.
pub fn valid_version(v: &str) -> bool {
    let v = v.strip_prefix('v').unwrap_or(v);
    let (core, pre) = v.split_once('-').map_or((v, None), |(c, p)| (c, Some(p)));
    let parts: Vec<&str> = core.split('.').collect();
    let numeric = |s: &str| !s.is_empty() && s.len() <= 6 && s.bytes().all(|b| b.is_ascii_digit());
    parts.len() == 3 && parts.iter().all(|p| numeric(p)) && pre.is_none_or(|p| p.strip_prefix("rc").is_some_and(numeric))
}

/// Télécharge l'installateur de `version` dans `dir`, vérifie son SHA-256 et renvoie son chemin.
pub fn download(version: &str, dir: &Path) -> Result<PathBuf, String> {
    if !valid_version(version) {
        return Err(format!("Refused: invalid version {version}"));
    }
    let version = version.trim_start_matches('v');
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let file = dir.join(format!("kleaner-setup-{version}.exe"));
    let base = format!("{RELEASES}/v{version}/kleaner-setup.exe");
    let path = file.display().to_string().replace('\'', "''");
    // Empreinte publiée par release.yml : « <sha256>  kleaner-setup.exe ». Fichier supprimé si elle ne correspond pas.
    let script = format!(
        "$ErrorActionPreference = 'Stop';
        Invoke-WebRequest -UseBasicParsing -Uri '{base}' -OutFile '{path}';
        $sum = (Invoke-WebRequest -UseBasicParsing -Uri '{base}.sha256').Content;
        if ($sum -is [byte[]]) {{ $sum = [Text.Encoding]::ASCII.GetString($sum) }}
        $want = ($sum.Trim() -split '\\s+')[0].ToLower();
        $got = (Get-FileHash -LiteralPath '{path}' -Algorithm SHA256).Hash.ToLower();
        if ($want -ne $got) {{ Remove-Item -LiteralPath '{path}' -Force; throw \"SHA-256 mismatch\" }}
        'OK'"
    );
    let (out, err, ok) = ps_run(&script)?;
    if ok && out.lines().last() == Some("OK") && file.is_file() {
        Ok(file)
    } else {
        Err(if err.is_empty() { "Download failed".into() } else { err })
    }
}

/// Lance l'installateur téléchargé en mode automatique (il demande lui-même les droits admin) et l'écrit au journal.
pub fn launch(installer: &Path, version: &str, journal_dir: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let w = |s: &std::ffi::OsStr| s.encode_wide().chain([0]).collect::<Vec<u16>>();
    let (verb, file, args) = (w("open".as_ref()), w(installer.as_os_str()), w("/P /UPDATE /R".as_ref()));
    let r = unsafe { ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(), args.as_ptr(), std::ptr::null(), SW_SHOWNORMAL) };
    let mut j = journal::Entry::new("update");
    j.paths = vec![version.to_string(), installer.display().to_string()];
    if r as isize <= 32 {
        j.error = Some("Could not start the installer".into());
    }
    journal::append(journal_dir, &j).map_err(|e| format!("Log: {e}"))?;
    if r as isize > 32 { Ok(()) } else { Err("Could not start the installer".into()) }
}

#[cfg(test)]
mod tests {
    use super::valid_version;

    #[test]
    fn only_plain_versions_reach_the_url() {
        for ok in ["1.1.0", "v1.1.0", "1.2.0-rc3", "v10.20.30"] {
            assert!(valid_version(ok), "{ok}");
        }
        for bad in ["", "1.1", "1.1.0.0", "1.1.0-beta", "1.1.0-rc", "1.1.0/../../evil", "1.1.0?x=1", "1.1.0 & calc", "a.b.c"] {
            assert!(!valid_version(bad), "{bad}");
        }
    }

    /// Sur la vraie machine (réseau) : `cargo test --release updater::tests::real_download -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_download() {
        let dir = std::env::temp_dir().join("kleaner-update-test");
        let f = super::download("1.0.0", &dir).unwrap();
        println!("{} ({} octets)", f.display(), std::fs::metadata(&f).unwrap().len());
        assert!(super::download("0.0.1", &dir).is_err(), "version inexistante : erreur, rien de lancé");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
