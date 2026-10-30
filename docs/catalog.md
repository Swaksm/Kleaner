# What Kleaner can clean

[Français](catalog.fr.md) · [Back to the README](../README.md)

Everything Kleaner's built-in catalog can permanently delete (46 items). Items for software you don't have are hidden. "Admin": requires the full scan.

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
| Compress Windows (CompactOS) | Windows system files, compressed in place by Windows' own tool (compact /CompactOS:always). | Windows may be slightly slower on an old PC. Undo with compact /CompactOS:never. | yes |
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
| uv: cache | Python packages downloaded and built by uv. | The next install downloads what is missing again. |  |
| Poetry: cache | Python packages already downloaded by Poetry. | The next install downloads what is missing again. |  |
| Cypress: binaries | The Cypress test runner downloaded for your projects. | Cypress downloads again the next time you run tests. |  |
| Graphics driver installers | Driver packages extracted by NVIDIA and AMD installers (C:\NVIDIA, C:\AMD, NVIDIA downloader). | Nothing. A driver reinstall downloads them again. | yes |

Browser and app caches only target cache folders (`Cache`, `Code Cache`, `GPUCache`, `cache2`…). A test fails the build if a catalog path touches `Cookies`, `Local Storage`, `IndexedDB`, `Login Data`, `Session Storage`, `Network`, `Service Worker`.

On top of this list, a **community catalog** adds about 1100 caches, temporary files and logs of other programs. It comes from [Winapp2](https://github.com/MoscaDotTo/Winapp2), filtered by Kleaner (see [WINAPP2-NOTICE.md](../WINAPP2-NOTICE.md)). Those items are never ticked for you.

Want an app added? Open an issue with the cache folder and a source showing it is safe to delete.
