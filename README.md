# Kleaner

**English** · [Français](README.fr.md)

Free, open-source and **transparent** Windows cleaner. Kleaner scans your drives, hidden files included, and removes what you don't usually see: temporary files, caches, update leftovers, crash dumps… **without changing how you use your PC**: you stay logged in everywhere and lose no files or settings.

## What Kleaner does

- **Scans** the drives you choose (internal, USB…): free space, largest files and folders, breakdown by type (charts), sortable by size, name or date.
- **Tells you if deleting something breaks it**: every large file and folder gets a verdict (safe, clean with its tool, uninstall, breaks something, your files, system, unknown) with what it is, what happens if you delete it, and how to free the space properly. Rules are hand-written in [`knowledge.json`](src-tauri/knowledge.json); Kleaner never guesses. Critical files ask for an extra confirmation.
- **Duplicates**: finds identical photos, videos, documents and archives (compared byte by byte) and keeps one copy.
- **Old projects**: finds `node_modules`, `.venv`, `target`, `build`… in projects you haven’t touched for months, with the command that rebuilds them.
- **Disk map**: every folder as a rectangle sized by its space; click to go inside.
- **Photos & videos**: lists every personal photo and video, sortable and searchable, with a list or grid view and a built-in viewer; send the ones you no longer need to the Recycle Bin.
- **Cleans** the items of the [catalog](#catalog): each one says what it removes, why it is useless and what changes for you. Nothing is ticked in advance, and « Preview the exact list » shows every file before anything is deleted.
- **Uninstalls** programs by launching their official uninstaller, with real size on disk and last launch date.
- **AI report** (optional, offline): generates a Markdown summary + prompt you paste into the AI of your choice; it asks you questions ("do you still use…?") and tells you what to remove. Kleaner sends nothing.
- **Logs** every action to `%LOCALAPPDATA%\Kleaner\journal.log`.
- **Admin mode** (on demand): full scan including protected system folders, read-only.

## What Kleaner never does

- Act without your confirmation.
- Touch your passwords, cookies, sessions or history.
- Write to the registry.
- Connect to the Internet: **no telemetry, no automatic updates**.
- Ask for admin rights unless you click "Full scan (admin)".
- Permanently delete a file outside the folders listed in [`src-tauri/cleaners.json`](src-tauri/cleaners.json). This is enforced in Rust ([`guard.rs`](src-tauri/src/guard.rs)) and covered by tests. Personal files only ever go to the Recycle Bin.

## Download

1. Go to [Releases](https://github.com/Swaksm/Kleaner/releases) and download `kleaner.exe` and `kleaner.exe.sha256`.
2. Check the hash (PowerShell):

```powershell
(Get-FileHash kleaner.exe -Algorithm SHA256).Hash.ToLower()
```

   It must match the content of `kleaner.exe.sha256`.
3. Run `kleaner.exe`. Nothing to install: it is a portable exe.

Or with a package manager:

```powershell
scoop bucket add kleaner https://github.com/Swaksm/Kleaner
scoop install kleaner/kleaner
```

(winget: coming soon, see [docs/distribution.md](docs/distribution.md).)

**Windows warning (SmartScreen).** The exe is not signed (a certificate is expensive for a free project). Windows shows "Windows protected your PC": click *More info* then *Run anyway*. Some antivirus tools may also react, because Kleaner deletes files and can ask for admin rights. All the code is readable here, and the exe is built by GitHub Actions from this code.

Requires 64-bit Windows 10 or 11 (WebView2, included in Windows 11 and up-to-date Windows 10).

## Removing Kleaner

*Settings → Remove Kleaner* erases `%LOCALAPPDATA%\Kleaner` and `%LOCALAPPDATA%\com.swaksm.kleaner`. Then delete `kleaner.exe`. Nothing else was installed.

## Catalog

Everything Kleaner can permanently delete. Items for software you don't have are simply hidden. "Admin": requires the full scan.

| Item | Removes | What changes | Admin |
|---|---|---|---|
| Temporary files | Files left in %TEMP% by installers and apps. | Nothing. |  |
| Thumbnail cache | Image and icon previews stored by File Explorer. | Thumbnails appear a little slower the first time. |  |
| Windows Internet cache | Copies of web pages and files kept by Windows and older apps. | Nothing. |  |
| Crash dumps | Memory copies of apps that crashed, and Windows error reports. | Nothing. |  |
| Graphics card caches | Shaders compiled by DirectX and NVIDIA or AMD drivers. | Games take a little longer to start the first time. |  |
| Recycle Bin | All files currently in the Recycle Bin, on every drive. | Irreversible: these files can no longer be restored. |  |
| Google Chrome: cache | Local copies of pages, scripts and shaders already loaded. | You stay logged in. Sites load a little slower the first time. |  |
| Microsoft Edge: cache | Local copies of pages, scripts and shaders already loaded. | You stay logged in. Sites load a little slower the first time. |  |
| Brave: cache | Local copies of pages, scripts and shaders already loaded. | You stay logged in. Sites load a little slower the first time. |  |
| Opera: cache | Local copies of pages and scripts already loaded. | You stay logged in. Sites load a little slower the first time. |  |
| Firefox: cache | Local copies of pages and files already loaded (cache2 folder). | You stay logged in. Sites load a little slower the first time. |  |
| Discord: cache | Images, videos and scripts already shown in Discord. | You stay logged in. Images appear a little slower. |  |
| Slack: cache | Images, files and scripts already shown in Slack. | You stay logged in. Images appear a little slower. |  |
| Microsoft Teams: cache | Images, files and scripts already shown in Teams (classic and new). | You stay logged in. First start takes a little longer. |  |
| VS Code: cache | Precompiled scripts and already-installed extension packages. | First start takes a little longer. |  |
| npm: cache | npm packages already downloaded, kept as copies. | The next install or build downloads what is missing again. |  |
| Yarn: cache | Yarn packages already downloaded, kept as copies. | The next install or build downloads what is missing again. |  |
| pip: cache | Python packages already downloaded by pip. | The next install or build downloads what is missing again. |  |
| NuGet: packages and cache | .NET packages downloaded by NuGet. | The next install or build downloads what is missing again. |  |
| Gradle: cache | Dependencies and build outputs kept by Gradle. | The next install or build downloads what is missing again. |  |
| Cargo: cache | Rust crates downloaded and unpacked by Cargo. | The next install or build downloads what is missing again. |  |
| Windows temporary files | Files left in C:\Windows\Temp by Windows and installers. | Nothing. | yes |
| System crash reports | Windows error reports and small memory dumps after a blue screen. | Nothing. | yes |
| Full memory dump | The C:\Windows\MEMORY.DMP file created after a blue screen. | Nothing. | yes |
| Delivery Optimization cache | Pieces of Windows updates kept to share with other PCs. | Nothing. | yes |
| Windows Update downloads | Update files already downloaded to SoftwareDistribution\Download. | A pending update will be downloaded again. | yes |
| Old Windows logs | Old Windows component setup logs (CbsPersist) older than 7 days. | Nothing. | yes |
| Previous Windows version | The Windows.old, $Windows.~BT and $Windows.~WS folders left after a major update. | Irreversible: you will no longer be able to roll back to the previous Windows version. | yes |
| Old Windows components (WinSxS) | Old component versions replaced by updates. | Already installed updates can no longer be uninstalled. | yes |
| Hibernation | The hiberfil.sys file, sized like a large part of your RAM. | Hibernation and fast startup turned off. Turn back on with powercfg /hibernate on. | yes |
| Old restore points | All system restore points except the most recent one. | You can no longer go back further than the latest point. | yes |
| Vivaldi: cache | Local copies of pages, scripts and shaders already loaded. | You stay logged in. Sites load a little slower the first time. |  |
| Chromium: cache | Local copies of pages, scripts and shaders already loaded. | You stay logged in. Sites load a little slower the first time. |  |
| Steam: web and shader cache | Store pages already viewed and shaders compiled for your games. | Games may stutter a little the first time they start. |  |
| Epic Games Launcher: web cache | Store pages and images already loaded by the launcher. | You stay logged in. The store loads a little slower once. |  |
| Adobe: media cache | Audio and video preview files built by Premiere Pro, After Effects and Media Encoder. | Projects take longer to open the first time (re-conforming). |  |
| Maven: cache | Packages and build outputs already downloaded by Maven. | The next install or build downloads what is missing again. |  |
| Go: cache | Packages and build outputs already downloaded by Go. | The next install or build downloads what is missing again. |  |
| Bun: cache | Packages and build outputs already downloaded by Bun. | The next install or build downloads what is missing again. |  |
| Composer: cache | Packages and build outputs already downloaded by Composer. | The next install or build downloads what is missing again. |  |
| Dart/Flutter (pub): cache | Packages and build outputs already downloaded by Dart/Flutter (pub). | The next install or build downloads what is missing again. |  |
| Graphics driver installers | Driver packages extracted by NVIDIA and AMD installers (C:\NVIDIA, C:\AMD, NVIDIA downloader). | Nothing. A driver reinstall downloads them again. | yes |

Browser and app caches only target cache folders (`Cache`, `Code Cache`, `GPUCache`, `cache2`…). A test fails the build if a catalog path touches `Cookies`, `Local Storage`, `IndexedDB`, `Login Data`, `Session Storage`, `Network`, `Service Worker`. Want an app added? Open an issue with the cache folder and a source showing it is safe to delete.

## Development

```bash
npm install
npm run tauri dev
npm test
npm run lint
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri build -- --no-bundle
```

Requirements: Node 22+, stable Rust, Visual Studio Build Tools (C++). Specification: [SPEC.md](SPEC.md) (French). Before each release: [docs/release-checklist.md](docs/release-checklist.md).

## Credits

Made by [swaks](https://github.com/Swaksm). MIT [license](LICENSE), no warranty.
