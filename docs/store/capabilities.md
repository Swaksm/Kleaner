# Capacités restreintes : justification pour Microsoft

Partner Center → Submission → Packages → « Why does your app need these restricted capabilities? ».
Champ unique, en anglais. Texte à copier :

---

Kleaner is a free, open-source (MIT) disk cleaner and system maintenance tool for Windows (source: https://github.com/Swaksm/Kleaner). It is a Win32 desktop app (Tauri/Rust) packaged as MSIX.

runFullTrust: Kleaner is a full-trust Win32 desktop application. It needs to read the whole file system to measure disk usage, and to delete caches and temporary files of other applications (browsers, Windows, games, developer tools).

allowElevation: Kleaner starts as a standard user and never elevates on its own. Only when the user clicks "Full scan (admin)" or picks a task that requires it, Kleaner relaunches itself through Windows' UAC prompt. Administrator rights are needed to: read system folders to measure their size (read-only, with the backup privilege); read the NTFS master file table for a fast read-only scan; clean system-wide caches such as the Windows Update download cache, Delivery Optimization files, Windows error reports and C:\Windows\Temp; run Windows' own tools when the user asks (DISM component store cleanup, removing Windows.old, old restore points, turning hibernation off); read boot durations from the Windows event log; and run the official uninstaller of a program installed for all users. Every action asks for confirmation first and is written to a readable local log.

unvirtualizedResources: Kleaner's job is to clean the real user profile. With write virtualization, its deletions in %LOCALAPPDATA% and %TEMP% and its registry changes would go to a private copy and have no effect. Kleaner writes to the registry only on explicit user request: turning a startup item on or off (the standard StartupApproved value that Task Manager also uses) and removing a dead entry from the installed programs list, after saving a .reg backup. It never cleans or "repairs" the registry. Its own data (settings and log) is kept in %LOCALAPPDATA%\Kleaner so that it stays visible and readable by the user.

Kleaner collects no data, has no telemetry and no account. Privacy policy: https://swaks.org/kleaner/privacy/

---

## Notes

- Délai de revue plus long avec des capacités restreintes ; Microsoft peut demander une démo vidéo.
- `unvirtualizedResources` : données de Kleaner (`%LOCALAPPDATA%\Kleaner`, `%LOCALAPPDATA%\com.swaksm.kleaner`) gardées après désinstallation par le Store (« Retirer Kleaner » dans les Paramètres les efface).
- Si Microsoft refuse `unvirtualizedResources` : sans elle, les écritures dans des fichiers déjà existants d'AppData passent quand même, mais les nouveaux fichiers et le registre HKCU vont dans une copie privée (démarrage, entrées fantômes, journal visible ailleurs).
