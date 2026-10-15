# Winapp2 notice

`src-tauri/winapp2.json` is an adaptation of **Winapp2.ini**, a community database of cleaning routines.

- **Original work:** [Winapp2](https://github.com/MoscaDotTo/Winapp2) by the Winapp2 project and its contributors (version 260915 at the time of the last import).
- **License of the original and of `src-tauri/winapp2.json`:** [Creative Commons Attribution-ShareAlike 4.0 International (CC BY-SA 4.0)](https://creativecommons.org/licenses/by-sa/4.0/).
- **The rest of Kleaner** (source code, `cleaners.json`, `knowledge.json`, documentation) is under the [MIT license](LICENSE). Only `winapp2.json` carries the ShareAlike terms.

## What was changed

Kleaner did not copy Winapp2.ini as is. `src-tauri/src/winapp2.rs` converts and filters it, and the result is committed as `winapp2.json`:

- Only file entries (`FileKey`) are kept. Registry entries (`RegKey`) are dropped: Kleaner does not clean the registry.
- Only folders that are clearly disposable are kept: caches, temporary files, logs, crash dumps and thumbnails, in AppData, LocalLow, hidden profile folders and ProgramData.
- Everything that could touch your data or your sessions is dropped: cookies, history, passwords, bookmarks, autofill, local storage, sessions, saves, downloads, documents, backups, security software, and the cache of Google Drive and Dropbox.
- Entries that Winapp2 itself does not tick by default, carry a warning, or use exclusions are dropped.
- Entries already covered by Kleaner's own catalog are dropped to avoid duplicates.
- Without the `RECURSE` or `REMOVESELF` flag, Kleaner does not go into sub-folders (`flat`), like Winapp2.
- The filter is applied again when Kleaner starts, so a hand-edited file cannot widen what is deleted.

## Updating

```bash
WINAPP2_INI=<path to Winapp2.ini> cargo test --manifest-path src-tauri/Cargo.toml --release winapp2::tests::regenerate -- --ignored --nocapture
cargo test --manifest-path src-tauri/Cargo.toml
```

Kleaner makes no network request for this: the file is converted by a developer and shipped inside the exe.
