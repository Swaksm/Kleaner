# Kleaner — notes pour les sessions Claude

Nettoyeur Windows gratuit et open source (MIT), par swaks. Exe portable Tauri 2 (Rust) + React/TypeScript.
Spécification : `SPEC.md` (révisions 1 à 6 en fin de fichier). README généré : `node scripts/gen-readme.cjs .`

## Règles non négociables

- **Aucune attribution Claude** dans les commits, PR, fichiers (pas de `Co-Authored-By`).
- **Aucun réseau**, sauf la vérification de mise à jour optionnelle (désactivée par défaut, `api.github.com`, autorisée dans la CSP). Windows Update et winget ne contactent Internet que sur clic.
- **Suppression définitive** seulement dans les dossiers de `src-tauri/cleaners.json` et `src-tauri/winapp2.json` (vérifié par `guard::check_delete`), ou les dossiers régénérables de projets trouvés par la dernière analyse (`delete_artifacts`).
- **Fichiers personnels et restes de programmes** : Corbeille uniquement (`guard::check_trash`, restaurable depuis le Journal).
- **Registre** : aucun nettoyage ni « réparation ». Seulement, sur demande et réversible : `StartupApproved` (démarrage) et la suppression d'une entrée fantôme de la liste des programmes, après sauvegarde `.reg` (SPEC, révision 8).
- **Liens et jonctions jamais suivis.** Fichiers verrouillés ignorés et comptés.
- **Chaque action qui modifie le disque** écrit une ligne dans `%LOCALAPPDATA%\Kleaner\journal.log`.
- **Les commandes qui suppriment** n'acceptent que des éléments trouvés par la dernière liste côté Rust (`ScanState` : artifacts, leftovers, cloud, apps, startup, shortcuts, ghosts), jamais un chemin libre envoyé par l'interface.
- **Kleaner ne devine jamais** : sans règle, le verdict est « inconnu ».

## Architecture

`src-tauri/src/` :
- `lib.rs` : commandes Tauri, `ScanState`, instance unique au démarrage.
- `guard.rs` : listes d'autorisation et de blocage, canonicalisation, refus des jonctions et des fichiers système.
- `scan.rs` : parcours parallèle (rayon). Produit top fichiers et dossiers, catégories, médias, doublons candidats, artifacts, arbre ≥ 64 Mo et espace des disques. Lecture en mode sauvegarde (`SeBackupPrivilege`) quand Kleaner est admin.
- `catalog.rs` + `cleaners.json` : nettoyages (mode `delete` ou `command`, niveau `safe` ou `heavy`), estimation, simulation (`preview`), `delete_project_dirs`.
- `knowledge.rs` + `knowledge.json` : verdicts. Ordre : règles précises → catalogue → programme propriétaire → règles générales → inconnu. Jokers `*`, `?`, `**`, captures `{1}`.
- `winapp2.rs` + `winapp2.json` : catalogue communautaire (Winapp2, CC BY-SA 4.0), converti et filtré (`entry_ok`), fusionné dans `catalog::all()` ; `catalog::base()` = le catalogue écrit à la main. Voir `WINAPP2-NOTICE.md`.
- `mft.rs` : scan rapide NTFS (admin) : lit la MFT en lecture seule et alimente `scan::walk` ; retombe sur le parcours classique au moindre doute ; `KLEANER_NO_MFT=1` le coupe. Test réel (terminal admin) : `mft::tests::real`.
- `duplicates.rs` : taille → échantillon → blake3, contenu personnel seulement.
- `leftovers.rs` : restes de programmes désinstallés (AppData, Programs, dossiers d'outils ; Program Files et ProgramData signalés seulement).
- `programs.rs` : registre, taille réelle, dernier lancement (Prefetch, admin), désinstalleur officiel.
- `actions.rs` : Corbeille (fichiers, dossiers), restauration, rapport de bug, rapport pour IA.
- `onedrive.rs` : « Libérer de l'espace » (attributs +Unpinned -Pinned).
- `system.rs` : infos machine, Windows Update (COM, lecture), winget (tableau lu), fin de support Windows.
- `elevation.rs` : admin, relance UAC (`--relaunch`), privilège de sauvegarde, verrou d'instance unique (`.Dev` en debug).
- `tray.rs` : en mode `--minimized` (démarrage auto), fenêtre cachée, icône près de l'horloge (Ouvrir / Quitter), notification si un disque fixe a moins de 10 % libre (une fois par 24 h). Textes dans la langue de Windows.
- `autostart.rs` : démarrage avec Windows sur demande (tâche planifiée « Kleaner », à l'ouverture de session, admin, `--minimized`), journalisé ; changer le réglage demande l'admin.
- `journal.rs` : journal JSON lignes.
- `startup.rs` : programmes au démarrage (Run, dossiers, tâches ; services en lecture seule), activation par `StartupApproved`.
- `shortcuts.rs` : raccourcis `.lnk` cassés.
- `health.rs` : santé et sécurité en lecture seule ; `PAGES` = liste fermée des écrans Windows ouvrables.
- `programs.rs` gère aussi les entrées fantômes (`ghosts`, `remove_ghost`, `restore_ghost`).

`src/` : `App.tsx` (navigation : essentiel / « Plus d'outils » / À propos + engrenage), `pages/*`, `components/*`, `lib/api.ts` (seul point de contact avec Rust), `lib/scanStore.ts` (dernier résultat partagé), `i18n/*.json` (6 langues : en, fr, es, de, it, pt ; textes du catalogue et des fiches en en/fr seulement, via `tl`).

## Commandes

```bash
npm test && npm run lint
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri build -- --no-bundle            # exe release
npm run tauri build -- --debug --no-bundle    # exe debug pour l'E2E
node e2e/run.mjs <kleaner.exe> <msedgedriver.exe>   # E2E sur le vrai exe (tauri-driver installé)
npm run demo:build && npm run demo:shots   # captures du README (mode demo, fausses données, Chrome sans fenêtre)
```

Mode demo (`src/demo/`, `vite --mode demo`) : remplace Tauri par des données fictives (vraies règles de `knowledge.json` pour les verdicts). Jamais dans l'exe. Paramètres d'adresse : `page`, `lang`, `theme`, `autoscan`, `scroll`. Modifier l'interface ou les règles : relancer les captures.

Cargo n'est pas dans le PATH de PowerShell : préfixer `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`.
Mesures sur la vraie machine : tests `#[ignore]` (`scan::tests::bench`, `catalog::tests::estimate_real`, `knowledge::tests::real`, `duplicates::real`, `leftovers::tests::real`, `system::tests::real`) avec `--release -- --ignored --nocapture`.

## Pièges déjà rencontrés

- **Échappement des `\`** : `node -e '…'` et les heredocs bash ont cassé plusieurs fois des chemins Windows (`\n`, `\r`, `\v` devenus des caractères de contrôle). Écrire les scripts dans un fichier, ou utiliser l'outil Edit. Les tests de `catalog`, `knowledge` et `i18n` refusent les caractères de contrôle.
- **YAML** : un nom d'étape contenant « : » doit être entre guillemets.
- **Corbeille** : Windows masque parfois l'extension dans le nom renvoyé, et le crate `trash` restaurait sans extension. `same_item` compare le dossier en forme longue (`RUNNER~1`) et remet le vrai nom.
- **serde** : PowerShell renvoie du PascalCase, d'où `rename_all(deserialize = "PascalCase")`. Ne pas sérialiser en PascalCase vers l'interface.
- **TypeScript** cible pré-ES2021 : pas de `replaceAll`.
- **Vite** garde parfois un module périmé : redémarrer le serveur de prévisualisation.
- **Règles proposées** : le verdict « inconnu » a un bouton qui ouvre le formulaire `.github/ISSUE_TEMPLATE/rule-proposal.yml` (chemin sans nom d'utilisateur) ; format des règles dans `docs/rules.md`.
- **Doublons** : jamais les .dll, .exe ou bases de données (MySQL, Flutter et Riot en double sont normaux).
- **Restes** : WSL, les pilotes (Realtek…) et les outils en ligne de commande (pnpm, gh, uv…) sont exclus. Android (SDK, `.android`) est proposé sans Android Studio mais avec un avertissement (Flutter s'en sert) : on avertit, on n'empêche pas.
- **Analyse** : pas de suppression depuis les listes de gros fichiers (verdict + « Ouvrir » seulement).

## Release

1. CI verte.
2. `git tag -a v0.1.0-rcN -m "v0.1.0-rcN" && git push origin v0.1.0-rcN`.
3. `release.yml` compile, signe si `SIGNPATH_API_TOKEN` existe, publie l'exe portable, l'installateur NSIS `kleaner-setup.exe` (par machine : Program Files, jamais `%LOCALAPPDATA%\Kleaner` qui est le dossier de données) et leurs SHA-256 (pré-release si le tag contient `-`).
4. Mettre à jour `bucket/kleaner.json` (version, URL, hash). winget : `node packaging/winget/make-manifests.mjs <version>` puis soumission manuelle (`docs/distribution.md`).

## En cours

- **SignPath** : refusé pour l'instant (il faut plus d'étoiles et de visibilité) ; README sans mention de signature. À redemander plus tard. Une fois acceptée : projet `kleaner`, politique `release-signing`, configuration `portable-exe`, secret et variable dans GitHub (`docs/signing.md`).
- **E2E en CI** : en lancement manuel. Sur les machines GitHub, WebView2 n'ouvre pas son port de débogage (« DevToolsActivePort file doesn't exist ») alors que l'exe démarre. En local, tout passe.
- **Jamais testés en vrai** : nettoyages lourds (Windows.old, DISM, hibernation, points de restauration), recherche Windows Update, mise à jour d'une appli par winget, restauration et OneDrive dans l'exe.
- **Scan rapide NTFS** : vérifié sur le vrai C: (2 s contre 8 à 21 s, écart 3 % d'espace dû aux recouvrements MSIX). Pas vérifié : volumes avec secteurs de 4 Ko, MFT très fragmentée, exFAT/ReFS (retombent sur le classique).
- **Feuille de route v0.3+ réalisée** (T18 à T22, `SPEC.md` révision 8) : démarrage, raccourcis cassés, santé et sécurité, entrées fantômes, Dropbox. Jamais testés dans l'exe : désactivation d'un élément de démarrage, suppression d'une entrée fantôme, durées de démarrage (admin). Pas de « nettoyage du registre » ni de « boosts ».
- **Installateur et assistant** (issue #2) : setup NSIS, assistant au premier lancement (`Setup.tsx` : langue, thème, mises à jour, démarrage avec Windows). Jamais testés en vrai : installation/désinstallation, création de la tâche de démarrage, lancement réduit.
- Dernière release : v0.1.0-rc13. Pas encore de v0.1.0 stable (nécessaire pour winget et le `checkver` de Scoop).
