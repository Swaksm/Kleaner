# Kleaner — notes pour les sessions Claude

Nettoyeur Windows gratuit et open source (MIT), par swaks. Exe portable Tauri 2 (Rust) + React/TypeScript.
Spécification : `SPEC.md` (révisions 1 à 6 en fin de fichier). README généré : `node scripts/gen-readme.cjs .`

## Règles non négociables

- **Aucune attribution Claude** dans les commits, PR, fichiers (pas de `Co-Authored-By`).
- **Aucun réseau**, sauf la vérification de mise à jour optionnelle (désactivée par défaut, `api.github.com`, autorisée dans la CSP). Windows Update et winget ne contactent Internet que sur clic.
- **Suppression définitive** seulement dans les dossiers de `src-tauri/cleaners.json` (vérifié par `guard::check_delete`), ou les dossiers régénérables de projets trouvés par la dernière analyse (`delete_artifacts`).
- **Fichiers personnels et restes de programmes** : Corbeille uniquement (`guard::check_trash`, restaurable depuis le Journal).
- **Liens et jonctions jamais suivis.** Fichiers verrouillés ignorés et comptés.
- **Chaque action qui modifie le disque** écrit une ligne dans `%LOCALAPPDATA%\Kleaner\journal.log`.
- **Les commandes qui suppriment** n'acceptent que des éléments trouvés par la dernière liste côté Rust (`ScanState` : artifacts, leftovers, cloud, apps), jamais un chemin libre envoyé par l'interface.
- **Kleaner ne devine jamais** : sans règle, le verdict est « inconnu ».

## Architecture

`src-tauri/src/` :
- `lib.rs` : commandes Tauri, `ScanState`, instance unique au démarrage.
- `guard.rs` : listes d'autorisation et de blocage, canonicalisation, refus des jonctions et des fichiers système.
- `scan.rs` : parcours parallèle (rayon). Produit top fichiers et dossiers, catégories, médias, doublons candidats, artifacts, arbre ≥ 64 Mo et espace des disques. Lecture en mode sauvegarde (`SeBackupPrivilege`) quand Kleaner est admin.
- `catalog.rs` + `cleaners.json` : nettoyages (mode `delete` ou `command`, niveau `safe` ou `heavy`), estimation, simulation (`preview`), `delete_project_dirs`.
- `knowledge.rs` + `knowledge.json` : verdicts. Ordre : règles précises → catalogue → programme propriétaire → règles générales → inconnu. Jokers `*`, `?`, `**`, captures `{1}`.
- `duplicates.rs` : taille → échantillon → blake3, contenu personnel seulement.
- `leftovers.rs` : restes de programmes désinstallés (AppData, Programs, dossiers d'outils ; Program Files et ProgramData signalés seulement).
- `programs.rs` : registre, taille réelle, dernier lancement (Prefetch, admin), désinstalleur officiel.
- `actions.rs` : Corbeille (fichiers, dossiers), restauration, rapport de bug, rapport pour IA.
- `onedrive.rs` : « Libérer de l'espace » (attributs +Unpinned -Pinned).
- `system.rs` : infos machine, Windows Update (COM, lecture), winget (tableau lu), fin de support Windows.
- `elevation.rs` : admin, relance UAC (`--relaunch`), privilège de sauvegarde, verrou d'instance unique (`.Dev` en debug).
- `journal.rs` : journal JSON lignes.

`src/` : `App.tsx` (navigation : essentiel / « Plus d'outils » / À propos + engrenage), `pages/*`, `components/*`, `lib/api.ts` (seul point de contact avec Rust), `lib/scanStore.ts` (dernier résultat partagé), `i18n/*.json` (6 langues : en, fr, es, de, it, pt ; textes du catalogue et des fiches en en/fr seulement, via `tl`).

## Commandes

```bash
npm test && npm run lint
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri build -- --no-bundle            # exe release
npm run tauri build -- --debug --no-bundle    # exe debug pour l'E2E
node e2e/run.mjs <kleaner.exe> <msedgedriver.exe>   # E2E sur le vrai exe (tauri-driver installé)
```

Cargo n'est pas dans le PATH de PowerShell : préfixer `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`.
Mesures sur la vraie machine : tests `#[ignore]` (`scan::tests::bench`, `catalog::tests::estimate_real`, `knowledge::tests::real`, `duplicates::real`, `leftovers::tests::real`, `system::tests::real`) avec `--release -- --ignored --nocapture`.

## Pièges déjà rencontrés

- **Échappement des `\`** : `node -e '…'` et les heredocs bash ont cassé plusieurs fois des chemins Windows (`\n`, `\r`, `\v` devenus des caractères de contrôle). Écrire les scripts dans un fichier, ou utiliser l'outil Edit. Les tests de `catalog`, `knowledge` et `i18n` refusent les caractères de contrôle.
- **YAML** : un nom d'étape contenant « : » doit être entre guillemets.
- **Corbeille** : Windows masque parfois l'extension dans le nom renvoyé, et le crate `trash` restaurait sans extension. `same_item` compare le dossier en forme longue (`RUNNER~1`) et remet le vrai nom.
- **serde** : PowerShell renvoie du PascalCase, d'où `rename_all(deserialize = "PascalCase")`. Ne pas sérialiser en PascalCase vers l'interface.
- **TypeScript** cible pré-ES2021 : pas de `replaceAll`.
- **Vite** garde parfois un module périmé : redémarrer le serveur de prévisualisation.
- **Doublons** : jamais les .dll, .exe ou bases de données (MySQL, Flutter et Riot en double sont normaux).
- **Restes** : exclure Android (SDK partagé par Flutter), WSL et les pilotes (Realtek…).

## Release

1. CI verte.
2. `git tag -a v0.1.0-rcN -m "v0.1.0-rcN" && git push origin v0.1.0-rcN`.
3. `release.yml` compile, signe si `SIGNPATH_API_TOKEN` existe, publie l'exe et son SHA-256 (pré-release si le tag contient `-`).
4. Mettre à jour `bucket/kleaner.json` (version, URL, hash). winget : `node packaging/winget/make-manifests.mjs <version>` puis soumission manuelle (`docs/distribution.md`).

## En cours

- **SignPath** : candidature envoyée (projet récent, refus possible). Une fois acceptée : projet `kleaner`, politique `release-signing`, configuration `portable-exe`, secret et variable dans GitHub (`docs/signing.md`).
- **E2E en CI** : en lancement manuel. Sur les machines GitHub, WebView2 n'ouvre pas son port de débogage (« DevToolsActivePort file doesn't exist ») alors que l'exe démarre. En local, tout passe.
- **Jamais testés en vrai** : nettoyages lourds (Windows.old, DISM, hibernation, points de restauration), recherche Windows Update, mise à jour d'une appli par winget, restauration et OneDrive dans l'exe.
- Dernière release : v0.1.0-rc9. Pas encore de v0.1.0 stable (nécessaire pour winget et le `checkver` de Scoop).
