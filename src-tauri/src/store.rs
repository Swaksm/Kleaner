//! Version Microsoft Store : le même exe, lancé depuis son paquet MSIX (`packaging/msix`).
//! Le Store gère les mises à jour et le démarrage avec Windows passe par la `StartupTask` du paquet.

use std::sync::OnceLock;

/// Identifiant de la tâche de démarrage, identique au `TaskId` de `packaging/msix/AppxManifest.xml`.
pub const STARTUP_TASK: &str = "KleanerStartup";

/// Kleaner tourne depuis son paquet MSIX (identité de paquet), donc installé par le Store.
pub fn is_packaged() -> bool {
    static PACKAGED: OnceLock<bool> = OnceLock::new();
    *PACKAGED.get_or_init(|| {
        use windows_sys::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
        const APPMODEL_ERROR_NO_PACKAGE: u32 = 15700;
        let mut len = 0u32;
        // Sans tampon : ERROR_INSUFFICIENT_BUFFER si un paquet existe, APPMODEL_ERROR_NO_PACKAGE sinon.
        unsafe { GetCurrentPackageFullName(&mut len, std::ptr::null_mut()) != APPMODEL_ERROR_NO_PACKAGE }
    })
}

/// Lancé par la tâche de démarrage du paquet (elle ne peut pas passer `--minimized`).
pub fn launched_at_startup() -> bool {
    use windows::ApplicationModel::Activation::ActivationKind;
    use windows::ApplicationModel::AppInstance;
    is_packaged() && AppInstance::GetActivatedEventArgs().and_then(|a| a.Kind()).is_ok_and(|k| k == ActivationKind::StartupTask)
}

/// Alias d'exécution du paquet (`kleaner.exe` dans `%LOCALAPPDATA%\Microsoft\WindowsApps`) : lancé par lui,
/// Kleaner garde son identité de paquet, même en admin. L'exe du dossier WindowsApps lancé directement la perdrait.
pub fn alias() -> Option<std::path::PathBuf> {
    let p = std::path::Path::new(&std::env::var_os("LOCALAPPDATA")?).join(r"Microsoft\WindowsApps\kleaner.exe");
    p.exists().then_some(p)
}

#[cfg(test)]
mod tests {
    #[test]
    fn tests_do_not_run_packaged() {
        assert!(!super::is_packaged());
        assert!(!super::launched_at_startup());
    }
}
