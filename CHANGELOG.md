# Changelog

Every change that users can see, newest first. Full release notes, downloads and SHA-256: [GitHub Releases](https://github.com/Swaksm/Kleaner/releases). The notes of each stable version are also in [docs/release-notes](docs/release-notes).

Versions follow [semantic versioning](https://semver.org/). Pre-releases (`-rc`) are test builds: they are listed under the version they led to.

## Unreleased

### Safety

- **Cleanups delete exactly what was checked**: each file is opened once, Kleaner asks Windows for the real location of that open file, checks it is inside the allowed folder, then deletes that same file. A folder replaced by a link between the check and the deletion can no longer send the deletion somewhere else. Compared with the previous method on a real PC: same decision for all 16,421 files.
- **Interrupted move repaired from the Journal**: if Kleaner stopped in the middle of moving a folder to another drive (shutdown, crash), the Journal offers “Check and repair”. Kleaner compares both folders file by file, then puts the original back, or finishes the move if the copy is already complete. Nothing is deleted without an identical copy elsewhere; if the folders differ, nothing is changed.
- More safety tests: symbolic links, device and volume paths (`\\.\`, `GLOBALROOT`), `\\?\` paths with `..`, NTFS streams, hard links, trailing spaces and dots, 8.3 short names.

## [1.3.2] - 2026-11-26

Pre-releases: 1.3.2-rc1, 1.3.2-rc2.

### Added

- **Startup**: an on/off switch for each program ([#4](https://github.com/Swaksm/Kleaner/issues/4)).
- **Journal**: actions grouped by day, one readable line each; click a line to see what was done, the files, the error or the command output ([#5](https://github.com/Swaksm/Kleaner/issues/5)).
- **Run as admin**: a button always visible under the Kleaner logo.
- **Privacy policy** linked next to the update check option.

### Fixed

- A Windows user name or a scheduled task name with a typographic apostrophe (’) broke the one-click update, “Start with Windows”, startup tasks and the broken shortcuts search.
- **Tips** no longer suggests Kleaner's own portable exe as an old installer.
- **Journal**: a refused restore point shows Windows' message instead of a technical line.
- Buttons aligned in Scan and on Home; no horizontal scrollbar in Tips.

## [1.3.1] - 2026-11-19

### Fixed

- **Program leftovers** go to the Recycle Bin one folder at a time: one blocked folder no longer cancels the others ([#3](https://github.com/Swaksm/Kleaner/issues/3)).
- **Recycle Bin too small**: Kleaner checks that the whole selection fits before anything moves.
- **Tick everything shown** and **Untick all** in Program leftovers, Tips and Ghost entries.
- **Free space updates by itself** after every cleanup, Recycle Bin or move.

## [1.3.0] - 2026-11-16

A safety release: a full audit of everything that touches the disk. Pre-releases: 1.2.1-rc1, 1.3.0-rc1.

### Added

- **Waiting window during actions**: progress, the app stays still until the end; a cleanup can be stopped.
- **Windows support date by edition** (Enterprise, Education, LTSC); Windows 11 26H1 and 26H2 known.
- Apps that update themselves (Epic Games, Steam, Battle.net, EA app, Ubisoft Connect, GOG Galaxy) are no longer sent to winget.

### Fixed

- **Windows.old** deleted without `takeown` and `icacls` (they followed folder links into the running Windows and failed on some languages).
- **OneDrive and Dropbox** folders are no longer taken for links.
- **Recycle Bin**: a file that Windows would delete permanently is refused before anything moves.
- **Standard Windows account**: admin mode refused with a clear message.
- **Move to another drive**: files created during the move are copied, `desktop.ini` folders can be moved, an interrupted move is written to the Journal.
- **Journal**: Restore buttons of older actions no longer disappear.
- **Start with Windows**: the portable exe no longer starts with admin rights; uninstalling removes the task; opening Kleaner again shows the hidden window.
- An unreachable network drive no longer freezes the window; F5 and right-click no longer reload the app.
- **Cleanups**: Dart/Flutter, Maven (never ticked automatically), `.venv` (only next to a package list), old restore points (the most recent one kept on each drive), new empty folders in `%TEMP%`.
- Health check texts translated in every language.

## [1.2.0] - 2026-10-30

Pre-releases: 1.2.0-rc1 to rc3.

### Added

- **Tips**: old installers and `.iso` files in Downloads, archives already extracted, Steam games not played for 6 months.
- **Preinstalled apps**: remove Candy Crush, TikTok, News, Weather, Xbox… for the current account only, from a closed list.
- **Restore point before heavy cleanups** (ticked by default).
- **CompactOS** heavy cleanup.
- **Filters in Scan** by name, verdict or age.
- **Monthly reminder** of what the risk-free cleanups would free (start with Windows).
- Slow pages load in the background and keep their results; long sections fold into one line.
- 16 new rules with their official source in the verdict card.

### Fixed

- The window no longer freezes while many files go to the Recycle Bin.
- Cleanup estimate several times faster.
- The “Unknown” filter no longer shows every row while verdicts load.
- Duplicates no longer offers files inside a code project.
- **Clean risk-free** on Home ticks the items and lets you check them before starting.

## [1.1.2] - 2026-10-29

### Fixed

- Portable version on a PC without Microsoft WebView2: a clear message in the language of Windows, with a link to Microsoft's installer.

## [1.1.1] - 2026-10-24

### Added

- **Guided tour** on first launch, replayable from Settings.
- **Updates without the setup wizard** (installed version): only a progress bar, then Kleaner reopens.

### Fixed

- A `target` folder next to a `Cargo.toml` is recognised as a Rust build folder.
- “Move…” refuses folders that are part of a code project or contain one.

## [1.1.0] - 2026-10-24

### Added

- **Russian and Chinese (simplified)**: 8 languages, file explanations and cleanup texts included.
- **One-click update** (installed version), SHA-256 checked.
- **Leftovers right after an uninstall** made from Kleaner.
- **Move a large folder to another drive**, checked file by file, undoable from the Journal.
- **Space freed so far** on Home.

## [1.0.0] - 2026-10-22

The first stable version.

### Added

- **Cleanups**: Windows temporary files, browser caches (you stay logged in), Windows Update cache, Recycle Bin, crash dumps, and over 1,100 programs from the community catalog Winapp2. Nothing ticked in advance; “Clean risk-free” shows the exact list first.
- **A verdict for every large file**: safe to delete, uninstall it, breaks something, your files, system or unknown (Kleaner never guesses).
- Program leftovers, duplicates, old code project folders, photo and video browser, disk map.
- Startup programs, broken shortcuts, ghost entries in the programs list, read-only health and security check-up, Windows and app updates.
- Optional start with Windows: an icon near the clock warns when a drive is almost full.
- Installer (`kleaner-setup.exe`) and portable exe; first-launch assistant (language, theme, updates, startup).

## [0.1.0 previews] - 2026-10-13 to 2026-10-22

Test builds 0.1.0-rc1 to rc18, before the first stable version.

- First scan of all drives, cleanup catalog with a safety guard (permanent deletion only inside known cache folders, personal files to the Recycle Bin), Journal of every action.
- “Does it break?” verdicts, programs with their real size and last launch, Photos & videos with a viewer, duplicates, old projects, disk map, cleanup simulation.
- Program leftovers, OneDrive “Free up space”, Restore from the Journal, bug report, optional update check, report for an AI.
- Six languages (English, French, Spanish, German, Italian, Portuguese), drive selection, admin scan.
- Installer, first-launch assistant, start with Windows ([#2](https://github.com/Swaksm/Kleaner/issues/2)), broken shortcuts fix ([#1](https://github.com/Swaksm/Kleaner/issues/1)), navigation icons, Mica effect, swaks.org colours.

[1.3.2]: https://github.com/Swaksm/Kleaner/releases/tag/v1.3.2
[1.3.1]: https://github.com/Swaksm/Kleaner/releases/tag/v1.3.1
[1.3.0]: https://github.com/Swaksm/Kleaner/releases/tag/v1.3.0
[1.2.0]: https://github.com/Swaksm/Kleaner/releases/tag/v1.2.0
[1.1.2]: https://github.com/Swaksm/Kleaner/releases/tag/v1.1.2
[1.1.1]: https://github.com/Swaksm/Kleaner/releases/tag/v1.1.1
[1.1.0]: https://github.com/Swaksm/Kleaner/releases/tag/v1.1.0
[1.0.0]: https://github.com/Swaksm/Kleaner/releases/tag/v1.0.0
[0.1.0 previews]: https://github.com/Swaksm/Kleaner/releases?q=0.1.0-rc&expanded=true
