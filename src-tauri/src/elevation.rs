//! Droits admin à la demande : Kleaner démarre en utilisateur et ne s'élève que sur clic.

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

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

/// Active le privilège de sauvegarde pour ce processus (admin uniquement).
/// Il permet de *lire* les dossiers réservés à SYSTEM (System Volume Information, WindowsApps…).
/// Kleaner ne s'en sert que pour lister des dossiers, jamais pour écrire.
pub fn enable_backup_privilege() -> bool {
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
        let name = wide("SeBackupPrivilege".as_ref());
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

fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    s.encode_wide().chain([0]).collect()
}

/// Relance l'exe en admin (fenêtre UAC de Windows). `Err` si l'utilisateur refuse.
pub fn relaunch_as_admin() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let verb = wide("runas".as_ref());
    let file = wide(exe.as_os_str());
    let r = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute renvoie une valeur > 32 en cas de succès.
    if r as isize > 32 { Ok(()) } else { Err("Admin rights refused or unavailable.".into()) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn is_elevated_does_not_crash() {
        let _ = super::is_elevated();
    }
}
