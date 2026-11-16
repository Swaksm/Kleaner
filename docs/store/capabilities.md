# Capacités restreintes : justification pour Microsoft

Partner Center → Submission → Packages → « Why does your app need these restricted capabilities? ».
Un champ par capacité, **500 caractères au plus** chacun (Submission options → Restricted capabilities), en anglais. Le détail va dans « Notes for certification » (voir plus bas, 6 000 caractères).

---

Kleaner is a free, open-source (MIT) disk cleaner and system maintenance tool for Windows (source: https://github.com/Swaksm/Kleaner). It is a Win32 desktop app (Tauri/Rust) packaged as MSIX.

runFullTrust: Kleaner is a full-trust Win32 desktop application. It needs to read the whole file system to measure disk usage, and to delete caches and temporary files of other applications (browsers, Windows, games, developer tools). On request, it also removes preinstalled Store apps for the current user only (Remove-AppxPackage), from a closed list that never includes the Store, Edge, Photos, Calculator, drivers or Windows components; each one can be reinstalled from the Store.

allowElevation: Kleaner starts as a standard user and never elevates on its own. Only when the user clicks "Full scan (admin)" or picks a task that requires it, Kleaner relaunches itself through Windows' UAC prompt. Administrator rights are needed to: read system folders to measure their size (read-only, with the backup privilege); read the NTFS master file table for a fast read-only scan; clean system-wide caches such as the Windows Update download cache, Delivery Optimization files, Windows error reports and C:\Windows\Temp; run Windows' own tools when the user asks (DISM component store cleanup, removing Windows.old, old restore points, turning hibernation off, CompactOS compression), after creating a system restore point first (ticked by default); read boot durations from the Windows event log; and run the official uninstaller of a program installed for all users. Every action asks for confirmation first and is written to a readable local log.

unvirtualizedResources: Kleaner cleans the real user profile: with write virtualization, its deletions in other apps' AppData caches and its HKCU changes would hit a private copy. HKCU writes, only after a user click: StartupApproved\Run and \StartupFolder values (Task Manager format), and deleting a dead Uninstall\<key> after a .reg backup. Virtualization disabled with desktop6, no flexible virtualization. Full answer to the 11/09/2026 report (exact paths, keys, uninstall) is in Notes for certification.

Kleaner collects no data, has no telemetry and no account. Privacy policy: https://swaks.org/kleaner/privacy/

---

## Notes for certification

Partner Center → Kleaner → Additional Testing Information (6 000 caractères au plus). Texte à copier :

---

Answer to the certification report of 11/09/2026 about the unvirtualizedResources capability.

1. Exact AppData / temp paths. Kleaner deletes files only inside folders listed in its public catalogs, and only after the user ticks them and confirms: src-tauri/cleaners.json (built-in, e.g. %TEMP%, %LOCALAPPDATA%\Microsoft\Windows\INetCache, %LOCALAPPDATA%\CrashDumps, %LOCALAPPDATA%\Google\Chrome\User Data\*\Cache, %APPDATA%\discord\Cache, %LOCALAPPDATA%\npm-cache, %LOCALAPPDATA%\pip\cache) and src-tauri/winapp2.json (community cache, temp and log folders of other apps under %LOCALAPPDATA%, %APPDATA% and %USERPROFILE%\AppData\LocalLow). A built-in check refuses any path outside these folders, links and junctions are never followed, and cookies, sessions, passwords and history are excluded by test. On user request it also sends to the Recycle Bin (restorable) folders of uninstalled programs under %LOCALAPPDATA%, %APPDATA%, %USERPROFILE%\AppData\LocalLow and %LOCALAPPDATA%\Programs, and broken .lnk shortcuts in %APPDATA%\Microsoft\Windows\Start Menu. Its own data is written to %LOCALAPPDATA%\Kleaner (journal.log, reports, .reg backups) and WebView2 uses %LOCALAPPDATA%\com.swaksm.kleaner. These caches belong to other apps: with write virtualization, new files would go to a private copy and the user would not find Kleaner's log and backups where the app tells them to look.

2. Exact HKCU registry keys and values, written only after a user click:
- HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run and ...\StartupApproved\StartupFolder: one REG_BINARY value per startup item, named after the item, in the same 12-byte format Task Manager writes (02 = enabled, 03 + date = disabled). Nothing is deleted from the Run key itself.
- HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\<subkey>: deleted only when it is a "ghost" entry (its uninstaller and install folder no longer exist), after "reg export" of the subkey to %LOCALAPPDATA%\Kleaner\backups\ghost-*.reg; "Restore" in Kleaner's log runs "reg import" of that file.
No other HKCU key or value is written. Kleaner never cleans or "repairs" the registry.

3. Virtualization declarations: desktop6:FileSystemWriteVirtualization = disabled and desktop6:RegistryWriteVirtualization = disabled. Flexible virtualization (virtualization:ExcludedDirectories / ExcludedKeys) is not used. If you prefer it, we can switch to flexible virtualization limited to the folders and the two keys above.

4. Other locations: no other file-system or HKCU location needs unvirtualized writes. Machine-wide changes (C:\Windows\Temp, the Windows Update download cache, HKLM StartupApproved values and HKLM Uninstall ghost entries) happen only after the user accepts a UAC prompt (allowElevation); they do not rely on unvirtualizedResources. The Store version starts with Windows through the package StartupTask; it creates no scheduled task.

5. After uninstall: deleted caches are regenerated by their apps; items sent to the Recycle Bin stay restorable from the Recycle Bin; a startup item turned off stays off until the user turns it back on in Task Manager or Settings > Apps > Startup; a removed ghost entry can be restored by double-clicking its .reg backup. Kleaner's own folders %LOCALAPPDATA%\Kleaner and %LOCALAPPDATA%\com.swaksm.kleaner remain: Settings > "Remove Kleaner" in the app deletes both before uninstalling, or the user can delete them by hand.

---

## Notes

- Délai de revue plus long avec des capacités restreintes ; Microsoft peut demander une démo vidéo.
- 11/09/2026 : 1.2.0 refusée, `unvirtualizedResources` « denied » (justification trop vague : chemins exacts, clés HKCU exactes, virtualisation flexible, autres emplacements, ce qui reste après désinstallation) ; `allowElevation` encore en examen. Le paragraphe `unvirtualizedResources` ci-dessus répond point par point.
- `unvirtualizedResources` : données de Kleaner (`%LOCALAPPDATA%\Kleaner`, `%LOCALAPPDATA%\com.swaksm.kleaner`) gardées après désinstallation par le Store (« Retirer Kleaner » dans les Paramètres les efface).
- Si Microsoft refuse `unvirtualizedResources` : sans elle, les écritures dans des fichiers déjà existants d'AppData passent quand même, mais les nouveaux fichiers et le registre HKCU vont dans une copie privée (démarrage, entrées fantômes, journal visible ailleurs).
