//! Droits admin à la demande : Kleaner démarre en utilisateur et ne s'élève que sur clic.

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

pub fn is_elevated() -> bool {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        );
        CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

/// Compte Windows standard (pas administrateur) : l'UAC demanderait le mot de passe d'un *autre* compte,
/// et Kleaner relancé tournerait avec ses dossiers (AppData, Corbeille, Documents), pas ceux de l'utilisateur.
pub fn is_standard_account() -> bool {
    use windows_sys::Win32::Security::{TOKEN_ELEVATION_TYPE, TokenElevationType, TokenElevationTypeDefault};
    if is_elevated() {
        return false;
    }
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut kind: TOKEN_ELEVATION_TYPE = 0;
        let mut len = 0u32;
        let ok = GetTokenInformation(token, TokenElevationType, (&mut kind as *mut TOKEN_ELEVATION_TYPE).cast(), size_of::<TOKEN_ELEVATION_TYPE>() as u32, &mut len);
        CloseHandle(token);
        // Administrateur avec UAC : jeton « limité ». Compte standard : jeton « par défaut » et pas élevé.
        ok != 0 && kind == TokenElevationTypeDefault
    }
}

/// Privilèges de sauvegarde et de restauration (admin uniquement).
/// Sauvegarde : *lire* les dossiers réservés à SYSTEM (System Volume Information, WindowsApps…) pour les lister.
/// Restauration : supprimer sans changer les droits ni le propriétaire (Windows.old, voir `catalog::commands`).
pub fn enable_backup_privilege() -> bool {
    enable_privilege("SeBackupPrivilege")
}

pub fn enable_privilege(privilege: &str) -> bool {
    use windows_sys::Win32::Foundation::{GetLastError, LUID};
    use windows_sys::Win32::Security::{
        AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
        TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
    };
    const ERROR_NOT_ALL_ASSIGNED: u32 = 1300;
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let name = wide(privilege.as_ref());
        let mut luid = LUID { LowPart: 0, HighPart: 0 };
        let mut ok = LookupPrivilegeValueW(std::ptr::null(), name.as_ptr(), &mut luid) != 0;
        if ok {
            let tp = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }],
            };
            ok = AdjustTokenPrivileges(token, 0, &tp, 0, std::ptr::null_mut(), std::ptr::null_mut()) != 0
                && GetLastError() != ERROR_NOT_ALL_ASSIGNED;
        }
        CloseHandle(token);
        ok
    }
}

pub const RELAUNCH_ARG: &str = "--relaunch";

/// Une seule fenêtre Kleaner à la fois. Renvoie `false` si une autre instance tourne déjà :
/// elle est alors ramenée au premier plan et l'appelant doit quitter.
/// Après une relance en admin, on attend jusqu'à 10 s que l'ancienne instance se ferme.
pub fn single_instance() -> bool {
    use windows_sys::Win32::Foundation::{WAIT_ABANDONED, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};
    let relaunch = std::env::args().any(|a| a == RELAUNCH_ARG);
    // Les builds de développement ont leur propre verrou : ils tournent à côté de la version installée.
    let lock = if cfg!(debug_assertions) { "Local\\Kleaner.SingleInstance.Dev" } else { "Local\\Kleaner.SingleInstance" };
    let name = wide(lock.as_ref());
    let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if mutex.is_null() {
        // Mutex créé par une instance admin, illisible depuis une instance utilisateur.
        focus_existing();
        return false;
    }
    let wait = unsafe { WaitForSingleObject(mutex, if relaunch { 10_000 } else { 0 }) };
    // Le mutex reste possédé jusqu'à la fin du processus : on ne ferme pas le handle.
    if wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED {
        return true;
    }
    focus_existing();
    false
}

fn show_event() -> Vec<u16> {
    wide(if cfg!(debug_assertions) { "Local\\Kleaner.Show.Dev" } else { "Local\\Kleaner.Show" }.as_ref())
}

/// Première instance : `on_show` est appelé chaque fois qu'une autre instance demande la fenêtre.
/// L'événement est ouvert à tous les processus de la session (liste d'accès vide) : une instance sans droits admin
/// peut ainsi réveiller une instance admin (démarrage avec Windows), ce que Windows refuse pour une fenêtre.
pub fn listen_show(on_show: impl Fn() + Send + 'static) {
    use windows_sys::Win32::Security::{InitializeSecurityDescriptor, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR, SetSecurityDescriptorDacl};
    use windows_sys::Win32::System::Threading::{CreateEventW, INFINITE, WaitForSingleObject};
    const SECURITY_DESCRIPTOR_REVISION: u32 = 1;
    let name = show_event();
    let event = unsafe {
        let mut sd: SECURITY_DESCRIPTOR = std::mem::zeroed();
        InitializeSecurityDescriptor((&mut sd as *mut SECURITY_DESCRIPTOR).cast(), SECURITY_DESCRIPTOR_REVISION);
        SetSecurityDescriptorDacl((&mut sd as *mut SECURITY_DESCRIPTOR).cast(), 1, std::ptr::null(), 0);
        let sa = SECURITY_ATTRIBUTES { nLength: size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: (&mut sd as *mut SECURITY_DESCRIPTOR).cast(), bInheritHandle: 0 };
        CreateEventW(&sa, 0, 0, name.as_ptr())
    };
    if event.is_null() {
        return;
    }
    // Le handle vit aussi longtemps que le processus.
    let event = event as usize;
    std::thread::spawn(move || {
        while unsafe { WaitForSingleObject(event as HANDLE, INFINITE) } == 0 {
            on_show();
        }
    });
}

/// Demande à la première instance de se montrer. `false` si elle n'écoute pas.
fn request_show() -> bool {
    use windows_sys::Win32::System::Threading::{EVENT_MODIFY_STATE, OpenEventW, SetEvent};
    let name = show_event();
    unsafe {
        let e = OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr());
        if e.is_null() {
            return false;
        }
        let ok = SetEvent(e) != 0;
        CloseHandle(e);
        ok
    }
}

fn focus_existing() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, SW_RESTORE, SetForegroundWindow, ShowWindow};
    if request_show() {
        return;
    }
    let title = wide("Kleaner".as_ref());
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
    }
}

fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    s.encode_wide().chain([0]).collect()
}

/// Relance l'exe en admin (fenêtre UAC de Windows). `Err` si l'utilisateur refuse.
pub fn relaunch_as_admin() -> Result<(), String> {
    if is_standard_account() {
        return Err("Refused: this Windows account is not an administrator (admin mode would run as another account)".into());
    }
    // Version Store : par l'alias du paquet, sinon l'instance admin perdrait son identité de paquet.
    let exe = match crate::store::is_packaged().then(crate::store::alias).flatten() {
        Some(alias) => alias,
        None => std::env::current_exe().map_err(|e| e.to_string())?,
    };
    // La nouvelle instance attend que celle-ci se ferme (voir `single_instance`).
    if crate::system::shell_execute("runas", exe.as_os_str(), RELAUNCH_ARG) { Ok(()) } else { Err("Admin rights refused or unavailable.".into()) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn is_elevated_does_not_crash() {
        let _ = super::is_elevated();
    }

    #[test]
    fn elevated_is_never_a_standard_account() {
        if super::is_elevated() {
            assert!(!super::is_standard_account());
        }
    }
}
