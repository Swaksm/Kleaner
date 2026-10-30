# Writing a rule

Kleaner explains files and folders with hand-written rules in [`src-tauri/knowledge.json`](../src-tauri/knowledge.json). Without a rule, an item is **unknown**: Kleaner never guesses.

You do not need to write code. In Kleaner, click the verdict of an unknown item, then **Propose a rule**: a GitHub form opens in your browser, pre-filled with the path (your user name hidden). You read it, complete it and decide whether to send it. Kleaner itself sends nothing.

## Format of a rule

```json
{
  "id": "example-cache",
  "match": ["%LOCALAPPDATA%\\Vendor\\App\\Cache"],
  "verdict": "safe",
  "what":        { "en": "Cache of Example.", "fr": "Cache d’Example." },
  "consequence": { "en": "Rebuilt automatically; the first launch is a little slower.", "fr": "Recréé automatiquement ; le premier lancement est un peu plus lent." },
  "howto":       { "en": "Delete it with Example closed.", "fr": "Supprime-le, Example fermé." }
}
```

- **`match`**: paths that start with `%LOCALAPPDATA%`, `%APPDATA%`, `%USERPROFILE%`, `%ProgramFiles%`… or `**\` or `?:\`. Wildcards: `*` (one folder level), `**` (any depth), `?`. A rule applies to the folder and everything inside it. `{1}` in the texts is replaced by what the first `*` matched.
- **`verdict`**: `safe` (regenerated cache, temp, logs), `cleanup` (Kleaner cleans it with its own tool), `uninstall`, `breaks`, `personal`, `system`.
- **`whole: true`** if the folder can only be deleted as a whole (a single file inside breaks it), like `node_modules` or a Python `.venv`.
- **`source`**: the official page (vendor, Microsoft) that confirms what the rule says. Shown as text in the verdict card, never opened by Kleaner. New rules need one; without an official source, the item stays « unknown ».
- Order of precedence: precise rules, then Kleaner's catalog, then the owner program, then generic rules (`"generic": true`), then unknown. A precise rule wins.

## Checks

```bash
cargo test --manifest-path src-tauri/Cargo.toml knowledge
```

The tests reject control characters, rules without text, and unknown cleaners. `common_paths_keep_their_verdict` pins the verdict of about 130 common folders: if your rule changes one of them, the test fails and shows which. Write the French text if you can; otherwise the English text is enough.

## Cleaning rules (what Kleaner deletes)

Deleting is separate and stricter: only folders listed in [`src-tauri/cleaners.json`](../src-tauri/cleaners.json) and the filtered community catalog ([`WINAPP2-NOTICE.md`](../WINAPP2-NOTICE.md)) can be deleted. A rule in `knowledge.json` never deletes anything. Open an issue with the cache folder and a source showing it is safe to delete.
