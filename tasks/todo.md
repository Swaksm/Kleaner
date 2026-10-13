# Tâches : Kleaner v1

## P0 — Socle

- [x] **T1 : Scaffold Tauri 2, React et TypeScript, plus git**
  - Acceptance : `npm run tauri dev` ouvre une fenêtre vide. `git init` est fait, le remote pointe vers `Swaksm/Kleaner`, le `.gitignore` est en place et la licence MIT est ajoutée.
  - Verify : `npm run tauri build` produit `kleaner.exe`. Noter sa taille.
  - Files : `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `.gitignore`, `LICENSE`

- [x] **T2 : Style guide, thème et i18n**
  - Acceptance : `tokens.css` contient les thèmes clair et sombre, avec la bascule mémorisée. Les polices sont embarquées. `useT` fonctionne avec `fr.json` et `en.json`, et la langue suit celle de Windows. La coquille de navigation contient Analyse, Nettoyage, Programmes, Journal et Paramètres. Les composants Card, StatusPill et Switch existent.
  - Verify : `npm test` (toutes les clés FR existent en EN, et inversement). Contrôle visuel des deux thèmes.
  - Files : `src/styles/tokens.css`, `src/i18n/*`, `src/App.tsx`, `src/components/*`

## P1 — Sécurité

- [x] **T3 : `guard.rs`**
  - Acceptance : `check_delete(path, roots)` refuse tout chemin hors des racines, ainsi que les variantes `..`, de casse, UNC et `\\?\`, et tout point de reparse. `check_trash(path)` applique la blocklist de la spec.
  - Verify : `cargo test guard`. Les tests créent une vraie jonction dans un dossier temporaire.
  - Files : `src-tauri/src/guard.rs`

- [ ] **T4 : `journal.rs`**
  - Acceptance : ajout d'une ligne JSON par action dans `%LOCALAPPDATA%\Kleaner\journal.log`, et lecture des N dernières lignes.
  - Verify : `cargo test journal`
  - Files : `src-tauri/src/journal.rs`

- [ ] **T5 : Avertissement, Journal et Paramètres**
  - Acceptance : l'avertissement bloque l'appli tant qu'il n'est pas accepté au premier lancement. La page Journal liste les actions. Les Paramètres permettent de changer le thème et la langue, et de retirer Kleaner (effacement de `%LOCALAPPDATA%\Kleaner`, dossier de données WebView2 redirigé dedans).
  - Verify : lancement manuel sur un profil vierge. Après « Retirer », le dossier n'existe plus.
  - Files : `src/pages/Journal.tsx`, `src/pages/Settings.tsx`, `src/components/Disclaimer.tsx`, `src-tauri/src/main.rs`

**Checkpoint P1** : `cargo test` et `npm test` passent. Rien de destructeur n'existe encore, à part « Retirer Kleaner ».

## P2 — Analyse

- [ ] **T6 : `scan.rs`**
  - Acceptance : le scan parcourt tous les disques fixes avec `jwalk`, fichiers cachés compris. Il émet des événements de progression, peut être annulé, et produit le top 50 des fichiers, le top 50 des dossiers et la taille par type (vidéos, images, archives, installeurs, autres). Les dossiers sans accès sont comptés à part.
  - Verify : `cargo test scan`, puis mesure sur la machine réelle (temps et RAM) consignée dans le PR.
  - Files : `src-tauri/src/scan.rs`, `src-tauri/src/main.rs`

- [ ] **T7 : Écran Analyse et actions sur les fichiers**
  - Acceptance : bouton Analyser, barre de progression, bouton Annuler. Résultats en cartes et graphique Recharts. Sur un fichier : « Ouvrir dans l'Explorateur » et « Corbeille » (sélection multiple, confirmation, `guard::check_trash`, journal).
  - Verify : `npm test` (confirmation). Test manuel sur un fichier factice.
  - Files : `src/pages/Analyse.tsx`, `src/components/ConfirmDialog.tsx`, `src/lib/api.ts`, `src-tauri/src/actions.rs`

**Checkpoint P2** : cible de performance tenue, ou écart documenté et corrigé.

## P3 — Nettoyage utilisateur

- [ ] **T8 : Moteur du catalogue (`catalog.rs`)**
  - Acceptance : charge `cleaners.json` embarqué. Le schéma exige `what`, `why` et `effect` en FR et en EN (140 caractères max), sinon le test échoue. Résout les variables d'environnement et les jokers. `estimate` donne la taille et le nombre de fichiers par entrée. `execute` supprime fichier par fichier via `guard`, sans traverser les liens, et ignore puis compte les fichiers verrouillés. Le champ `close` est respecté (processus actif = entrée ignorée).
  - Verify : `cargo test catalog` sur une arborescence temporaire, avec un fichier verrouillé et une jonction piégée.
  - Files : `src-tauri/src/catalog.rs`, `src-tauri/cleaners.json`

- [ ] **T9 : Entrées utilisateur Windows**
  - Acceptance : `%TEMP%`, miniatures, INetCache, CrashDumps, WER utilisateur, caches de shaders, Corbeille.
  - Verify : test qui valide tout le catalogue (racines, schéma). Estimation lancée sur la machine réelle.
  - Files : `src-tauri/cleaners.json`, `src-tauri/src/catalog.rs` (test)

- [ ] **T10 : Entrées navigateurs et applis**
  - Acceptance : Chrome, Edge, Brave, Opera, Firefox, Discord, Slack, Teams, VS Code et Spotify. Uniquement des dossiers de cache, chaque chemin justifié par un commentaire de source dans le PR.
  - Verify : un test fait échouer toute entrée qui cible `Cookies`, `Local Storage`, `IndexedDB`, `Login Data`, `Session Storage` ou `Service Worker`. Vérification sur VM : toujours connecté après le nettoyage.
  - Files : `src-tauri/cleaners.json`

- [ ] **T11 : Entrées outils dev**
  - Acceptance : caches npm, Yarn, pip, NuGet, Gradle et Cargo.
  - Verify : test du catalogue. `npm install` fonctionne toujours après le nettoyage.
  - Files : `src-tauri/cleaners.json`

- [ ] **T12 : Écran Nettoyage**
  - Acceptance : une carte par catégorie avec la taille estimée, « Ce que ça supprime », « Pourquoi c'est inutile », « Ce que ça change » et une case (cochée si `safe`). Ces trois lignes sont reprises dans la confirmation. La confirmation indique le total et le mode. Le rapport montre l'espace libéré et les fichiers ignorés, avec leur raison.
  - Verify : `npm test`, puis nettoyage réel sur la VM.
  - Files : `src/pages/Cleanup.tsx`, `src/components/CleanerCard.tsx`, `src/lib/api.ts`

**Checkpoint P3 (bloquant)** : sur la VM, après un nettoyage complet, l'utilisateur est toujours connecté partout.

## P4 — Admin

- [ ] **T13 : Élévation**
  - Acceptance : `is_elevated()`. « Analyse complète » relance l'exe via `runas` et ferme l'instance courante. En mode non admin, les entrées `admin` affichent « Analyse complète requise ».
  - Verify : test manuel de l'UAC, accepté puis refusé (dans ce cas, l'appli reste ouverte en mode utilisateur).
  - Files : `src-tauri/src/elevation.rs`, `src/pages/Cleanup.tsx`

- [ ] **T14 : Entrées système `safe`**
  - Acceptance : `C:\Windows\Temp`, WER système, Minidump, `MEMORY.DMP`, Delivery Optimization, cache Windows Update (arrêt de `wuauserv` et `bits`, suppression, puis relance garantie même en cas d'erreur), anciens logs CBS et DISM.
  - Verify : test du catalogue. Sur VM, Windows Update fonctionne toujours après le nettoyage.
  - Files : `src-tauri/cleaners.json`, `src-tauri/src/actions.rs`

- [ ] **T15 : Nettoyages `heavy`**
  - Acceptance : Windows.old (avec `$Windows.~BT` et `~WS`), DISM StartComponentCleanup, `powercfg /hibernate off` et points de restauration (tous sauf le plus récent). Chacun a une carte dédiée, affiche la commande exacte, demande une confirmation en deux étapes et enregistre sa sortie dans le journal.
  - Verify : sur un snapshot de VM, chaque action une par une, puis vérifier que Windows démarre et que la mise à jour fonctionne.
  - Files : `src-tauri/cleaners.json`, `src-tauri/src/actions.rs`, `src/components/CleanerCard.tsx`

## P5 — Programmes

- [ ] **T16 : Programmes installés**
  - Acceptance : lecture de HKLM et HKCU (32 et 64 bits), avec nom, éditeur, `EstimatedSize` et date. Tri par taille et recherche. « Désinstaller » lance l'`UninstallString` après confirmation, puis rafraîchit la liste. Les composants système (`SystemComponent=1`) sont masqués.
  - Verify : `cargo test programs` (parsing des chaînes). Désinstallation manuelle d'un petit programme sur VM.
  - Files : `src-tauri/src/programs.rs`, `src/pages/Programs.tsx`, `src/lib/api.ts`

## P6 — Release

- [x] **T0 : CI/CD de base** (faite en avance)
  - `ci.yml` : à chaque push ou PR, lint, tests, clippy et cargo test, sautés tant que `package.json` n'existe pas. `release.yml` : sur un tag `v*`, build de l'exe portable (`--no-bundle`), calcul du SHA-256 et publication dans la Release.
  - Files : `.github/workflows/ci.yml`, `.github/workflows/release.yml`

- [ ] **T17 : README et check-list** (CI déjà faite en T0, à vérifier avec un tag `v0.1.0-rc1`)
  - Acceptance : un workflow GitHub Actions (windows-latest) exécute tests, clippy et build sur chaque PR. Sur un tag `v*`, il publie l'exe et son SHA-256 dans Releases. Le README, en FR et en EN, explique ce que fait Kleaner, la liste du catalogue, l'absence de réseau, SmartScreen et la vérification du hash. `docs/release-checklist.md` est rempli.
  - Verify : un tag `v0.1.0-rc1` produit la release. Exe téléchargé, puis lancé sur une VM vierge.
  - Files : `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `README.md`, `docs/release-checklist.md`
