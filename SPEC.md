# Spec : Kleaner

> Statut : **validée le 2026-10-13, révision 2**. La révision 2 analyse tout le PC, ajoute les fichiers cachés, les droits admin à la demande, les caches et l'interface FR/EN.
> Dépôt : https://github.com/Swaksm/Kleaner

## Hypothèses

1. Windows 10/11 64 bits uniquement. WebView2 est présent (inclus dans Windows 11 et dans Windows 10 à jour).
2. Interface **FR + EN**. La langue suit celle de Windows et peut être changée dans les Paramètres.
3. Kleaner démarre **sans droits admin**. Le bouton « Analyse complète » le relance en admin, ce qui affiche la fenêtre UAC de Windows.
4. **Aucune connexion réseau**, aucune télémétrie, aucune mise à jour automatique.
5. **L'exe n'est pas signé.** SmartScreen et certains antivirus vont avertir, d'autant plus qu'il supprime des fichiers et demande les droits admin. Le README l'explique et publie le SHA-256 de chaque release.

## Objectif

Kleaner est une appli Windows gratuite et open source. Elle **réduit drastiquement l'espace occupé sur le PC**. Elle analyse tous les disques, y compris les fichiers cachés, et trouve tout ce qu'on ne voit pas d'habitude : temporaires, caches Windows, d'applis et de navigateurs, restes de mises à jour, dumps de plantage, Windows.old, hibernation, points de restauration.

Elle supprime tout cela **sans rien changer à la vie de l'utilisateur** : il reste connecté partout et ne perd ni données ni réglages. Elle ne fait **jamais rien dans son dos**.

Elle vise le grand public non technique : simple, rapide et légère.

### Récits utilisateur

- **Voir l'invisible.** Je lance l'exe et je clique sur « Analyser ». Kleaner analyse tous mes disques, fichiers cachés compris. Il affiche l'espace récupérable par catégorie, ainsi que mes plus gros fichiers et dossiers.
- **Nettoyer en un clic.** Je coche des catégories, et une confirmation m'indique la taille totale, le nombre de fichiers et ce que ça change (en général : rien). Je valide et je vois l'espace réellement libéré.
- **Aller plus loin.** Je clique sur « Analyse complète (admin) » et j'accepte l'UAC. Kleaner trouve alors aussi les temporaires système, le cache Windows Update, les dumps, Windows.old, etc.
- **Nettoyages lourds.** Windows.old, WinSxS, l'hibernation et les points de restauration ont chacun une carte dédiée. Elle explique clairement ce qu'on perd, et l'action demande sa propre confirmation.
- **Mes fichiers.** Depuis les plus gros fichiers, je peux ouvrir l'emplacement dans l'Explorateur ou envoyer le fichier à la Corbeille.
- **Désinstaller.** Je vois les programmes installés triés par taille, et je lance leur désinstalleur officiel.
- **Faire confiance.** Au premier lancement, un avertissement clair s'affiche et je dois l'accepter. Un journal liste tout ce que Kleaner a fait.
- **Retirer Kleaner.** Un bouton efface toutes les données de Kleaner, puis explique comment supprimer l'exe.

### Hors périmètre v1

Pas de nettoyage du registre, pas de « boost » ni d'optimisation, pas de doublons, pas de planification, pas de suppression de cookies, d'historique ou de mots de passe, pas de macOS ni de Linux.

## Le catalogue de nettoyage

Tout ce que Kleaner peut supprimer définitivement est décrit dans **un seul fichier** : `src-tauri/cleaners.json`, lisible sur GitHub. Ce qui n'y figure pas ne peut pas être supprimé définitivement.

Chaque entrée contient :

| Champ | Rôle |
|---|---|
| `id` | Identifiant stable, par exemple `browser-chrome-cache` |
| `label` | Nom court, en FR et en EN |
| `what` | **Ce que ça supprime**, en une phrase (« Copies locales des pages web déjà visitées ») |
| `why` | **Pourquoi c'est inutile**, en une phrase (« Le navigateur les retélécharge au besoin ») |
| `effect` | **Ce que ça change pour l'utilisateur** (« Rien » / « Sites un peu plus lents au 1er chargement » / « Hibernation désactivée ») |
| `paths` | Chemins avec variables d'environnement et jokers (`%LOCALAPPDATA%\Google\Chrome\User Data\*\Cache`) |
| `admin` | `true` si l'entrée demande les droits admin |
| `mode` | `delete` (vide le contenu des dossiers, garde le dossier) ou `command` (outil Windows officiel, géré par `id`) |
| `only` | Optionnel : seuls les fichiers dont le nom correspond (`thumbcache_*.db`) |
| `older_than_hours` | Optionnel : seuls les fichiers plus vieux que ça (`%TEMP%` : 24 h) |
| `level` | `safe` (proposé par « Cocher les éléments sans risque » ; rien n'est coché d'avance), `heavy` (carte dédiée et confirmation dédiée) |
| `close` | Processus qui doivent être fermés (`chrome.exe`…). S'ils tournent, l'entrée est ignorée et signalée |

### Contenu v1

**Utilisateur, sans admin, niveau `safe` :**
- `%TEMP%`
- Cache des miniatures
- INetCache
- CrashDumps et WER de l'utilisateur
- Caches de shaders DirectX, NVIDIA et AMD
- Corbeille
- **Navigateurs** (Chrome, Edge, Brave, Opera, Firefox) : uniquement les dossiers `Cache`, `Code Cache`, `GPUCache` et `cache2`
- **Applis** (Discord, Slack, Teams, VS Code) : uniquement leurs dossiers de cache. Spotify exclu : son cache peut contenir les titres téléchargés hors ligne.
- **Outils dev** : caches npm, Yarn, pip, NuGet, Gradle et Cargo

**Système, admin, niveau `safe` :**
- `C:\Windows\Temp`
- WER système
- Minidumps et `MEMORY.DMP`
- Cache Delivery Optimization
- Cache de téléchargement Windows Update (les services sont arrêtés puis relancés)
- Anciens logs CBS et DISM

**Système, admin, niveau `heavy` :**
- **Windows.old et `$Windows.~BT` / `$Windows.~WS`** : on ne peut plus revenir à la version précédente de Windows.
- **WinSxS** via `Dism.exe /Online /Cleanup-Image /StartComponentCleanup`. C'est sans perte, mais long.
- **Hibernation** via `powercfg /hibernate off` : on perd l'hibernation et le démarrage rapide.
- **Points de restauration** : tous sauf le plus récent, via `vssadmin delete shadows /shadow={id}`.

**Règle des caches :** ne jamais toucher aux cookies, `Local Storage`, `IndexedDB`, `Session Storage`, `Login Data`, `Service Worker`, ni aux contenus téléchargés hors ligne. Après le nettoyage, l'utilisateur doit rester connecté partout. Chaque entrée navigateur ou appli est vérifiée à la main avant d'entrer dans le catalogue.

## Règles de transparence et de sécurité (non négociables)

1. **Lecture seule par défaut.** L'analyse ne modifie rien.
2. **Rien sans confirmation.** Avant d'agir, Kleaner affiche le nombre de fichiers, la taille, la conséquence et le mode (« suppression définitive » ou « Corbeille »). Le détail des chemins est disponible.
2b. **Toujours expliquer, en bref.** Chaque carte et chaque confirmation affichent `what`, `why` et `effect` : trois phrases courtes (140 caractères max chacune), sans jargon. Une entrée sans ces trois textes en FR et en EN ne peut pas entrer dans le catalogue.
3. **Suppression définitive uniquement dans le catalogue.** La garde Rust (`guard.rs`) refuse toute suppression définitive hors des racines déclarées dans `cleaners.json`.
4. **Fichiers personnels : Corbeille uniquement.** Ce sont les gros fichiers choisis à la main. Ils sont refusés s'ils se trouvent sous `C:\Windows`, `Program Files*`, `ProgramData`, à la racine d'un disque, ou s'il s'agit d'un dossier utilisateur entier (Documents, Bureau…).
5. **Jamais de suivi des liens.** La suppression ne traverse jamais une jonction ni un lien symbolique, car un lien dans un cache pourrait pointer vers des documents.
6. **Fichier verrouillé = ignoré.** Un fichier en cours d'utilisation est passé et compté dans le rapport, sans forcer.
7. **Commandes Windows officielles uniquement** pour les nettoyages `command` (DISM, powercfg, vssadmin, gestion des services). La commande exacte est affichée avant exécution, et sa sortie est enregistrée dans le journal.
8. **Journal.** Chaque action (date, entrée, nombre de fichiers, taille libérée, fichiers ignorés, erreurs, sortie des commandes) est écrite dans `%LOCALAPPDATA%\Kleaner\journal.log`, lisible dans l'appli.
9. **Avertissement au premier lancement.** Il explique ce que fait Kleaner, ce qu'il ne fait jamais, la règle « aucun réseau », la suppression définitive des caches, le journal, et la licence MIT sans garantie. Il faut l'accepter pour continuer.
10. **Désinstallation** : uniquement via l'`UninstallString` du registre. Kleaner ne supprime jamais lui-même les fichiers d'un programme.

## Stack technique

- **Tauri 2** (cœur en Rust) + **React 19 + TypeScript + Vite**, **Recharts** pour les graphiques.
- Crates Rust : `rayon` (parcours parallèle maison, `read_dir` donne les tailles sans appel système en plus), `trash` (Corbeille), `winreg` (lecture des programmes installés), `windows-sys` (disques, élévation, vidage de la Corbeille, processus), `serde` et `serde_json`. Jokers du catalogue gérés à la main (`*`, `?`).
- i18n : deux fichiers `fr.json` / `en.json` et un petit hook React, sans librairie.
- Polices Archivo et IBM Plex Sans/Mono **embarquées** dans l'exe, sans Google Fonts.
- Pas de framework CSS : variables CSS du style guide.

## Commandes

```bash
npm install
npm run tauri dev
npm run tauri build                                   # -> src-tauri/target/release/kleaner.exe
cargo test --manifest-path src-tauri/Cargo.toml
npm test
npm run lint
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## Structure du projet

```
src/
  pages/                → Analyse, Nettoyage, Programmes, Journal, Paramètres
  components/           → Card, StatusPill, Switch, ConfirmDialog, SizeBar, CleanerCard
  i18n/                 → fr.json, en.json, useT.ts
  styles/tokens.css     → Variables du style guide (clair/sombre)
  lib/api.ts            → Appels typés aux commandes Tauri
src-tauri/
  cleaners.json         → Catalogue : tout ce qui peut être supprimé
  src/
    scan.rs             → Parcours de tous les disques, tops, catégories
    catalog.rs          → Chargement du catalogue, estimation, exécution
    guard.rs            → Allowlist (catalogue) + blocklist (Corbeille), refus des liens
    actions.rs          → Corbeille, ouvrir l'Explorateur, commandes Windows
    elevation.rs        → Détection de l'admin et relance via runas
    programs.rs         → Programmes installés (registre)
    journal.rs          → Journal
    main.rs
.github/workflows/      → Build et release sur tag v*
docs/release-checklist.md
SPEC.md, README.md, LICENSE (MIT)
```

## Style du code

```rust
#[tauri::command]
pub fn run_cleaner(id: String) -> Result<CleanReport, String> {
    let entry = catalog::get(&id).ok_or("Entrée inconnue")?;
    if entry.admin && !elevation::is_elevated() {
        return Err("Droits admin requis".into());
    }
    let report = catalog::execute(entry)?; // chaque chemin passe par guard::check_delete
    journal::log(&report)?;
    Ok(report)
}
```

```css
:root {
  --bg: #FBFAF7; --card: #FFFFFF; --sunken: #F2F0EA;
  --text: #1C1D1F; --muted: #6B6A63; --faint: #A3A099;
  --border: #E7E4DB; --border-strong: #D5D1C4;
  --accent: #35479E; --danger: #B6324A; --warn: #A6670F; --ok: #2F7D53;
  --radius-card: 10px; --radius-ctl: 7px;
}
:root[data-theme="dark"] {
  --bg: #15161B; --card: #1C1E24; --sunken: #0F1013;
  --text: #ECE9E1; --muted: #9B998F;
  --border: #2B2D33; --border-strong: #3B3E46; --accent: #7E8DF2;
}
```

Conventions :

- Fichiers React en `PascalCase.tsx`, modules Rust en `snake_case`.
- Tailles en IBM Plex Mono `tabular-nums`, en unités binaires.
- Thème initial selon `prefers-color-scheme`, puis mémorisé dans `localStorage`.
- Pas de dégradés ni d'ombres fortes.
- Statuts en pills avec un point de couleur.
- Aucun texte d'interface écrit en dur : tout passe par `useT`.

## Stratégie de test

- **Rust (`cargo test`)** : c'est la priorité.
  - `guard` : refuse tout chemin hors des racines du catalogue. Refuse les variantes `..`, de casse, UNC et `\\?\`. Refuse les jonctions et liens symboliques, y compris une jonction placée *dans* un dossier de cache.
  - `catalog` : `cleaners.json` est valide. Chaque entrée a `what`, `why` et `effect` non vides en FR et en EN, de 140 caractères maximum. Chaque chemin se résout sous une racine autorisée. Aucune entrée navigateur ou appli ne cible `Cookies`, `Local Storage`, `IndexedDB`, `Login Data`, `Session Storage` ni `Service Worker` (test qui parcourt tout le catalogue). L'estimation et l'exécution sont testées sur une arborescence temporaire, avec un fichier verrouillé ignoré et compté.
  - `scan` : tailles, tri, fichiers cachés inclus, dossier sans accès ignoré, annulation.
  - `journal` : écriture et lecture.
- **Front (Vitest + Testing Library)** : la confirmation affiche la taille, la conséquence et le mode. La confirmation `heavy` est en deux étapes. Toutes les clés existent en FR et en EN.
- **Manuel, sur VM Windows 10 et 11 avant chaque release** (`docs/release-checklist.md`) : on se connecte à Chrome, Discord et VS Code, on nettoie, puis **on vérifie qu'on est toujours connecté**. On teste aussi chaque nettoyage `heavy` sur un snapshot de VM.

## Limites de travail

- **Toujours** : faire passer toute suppression par `guard`, journaliser, faire passer tests et clippy avant un commit, ajouter un test pour toute nouvelle entrée du catalogue.
- **Demander d'abord** : ajouter ou modifier une entrée du catalogue, une dépendance hors stack, toute action réseau, la CI ou la licence.
- **Jamais** : supprimer hors du catalogue, toucher aux cookies, sessions ou identifiants, écrire dans le registre (sauf les deux exceptions de la révision 8, sur demande), s'élever sans clic de l'utilisateur, envoyer de la télémétrie, créer des fichiers hors de `%LOCALAPPDATA%\Kleaner`.

## Critères de réussite

1. `kleaner.exe` portable fait **moins de 15 Mo** et démarre en **moins de 2 s**.
2. L'analyse de tous les disques fixes, fichiers cachés compris, prend **moins de 60 s pour 1 million de fichiers** sur SSD. La progression est visible et un bouton Annuler est disponible. La RAM reste sous 250 Mo.
3. L'écran Nettoyage affiche l'espace récupérable par catégorie. Rien n'est coché d'avance ; un bouton coche les entrées `safe` utiles. Le rafraîchissement ne coche jamais rien. Le rapport après nettoyage indique la taille libérée et le nombre de fichiers ignorés.
4. Après un nettoyage complet sur la VM de test, **l'utilisateur est toujours connecté** dans Chrome, Edge, Firefox, Discord et VS Code (check-list manuelle).
5. Les tests prouvent que toute suppression hors catalogue est refusée, ainsi que toute traversée de lien.
6. La relance admin fonctionne. Sans admin, les entrées `admin` sont affichées avec « Analyse complète requise ».
7. Les 4 nettoyages `heavy` fonctionnent. Chacun montre la commande exacte et la conséquence, puis enregistre sa sortie dans le journal.
8. Les gros fichiers peuvent être ouverts dans l'Explorateur ou envoyés à la Corbeille.
9. La liste des programmes vient du registre (HKLM et HKCU, 32 et 64 bits), triée par taille, et le désinstalleur se lance.
10. Le journal est complet. « Retirer Kleaner » efface `%LOCALAPPDATA%\Kleaner` et les données WebView2.
11. L'avertissement s'affiche au premier lancement. L'interface est complète en FR et en EN, avec thème clair et sombre et focus clavier visible.
12. Aucun appel réseau : pas de permission HTTP et une CSP `connect-src` restreinte.
13. La CI publie l'exe et son SHA-256 dans GitHub Releases sur un tag `v*`.

## Décisions prises

- Exe non signé en v1, expliqué dans le README.
- Interface FR + EN.
- Droits admin à la demande.
- Caches supprimés définitivement, fichiers personnels envoyés à la Corbeille.
- Les 4 nettoyages `heavy` sont inclus.
- Caches navigateurs, applis, dev et GPU inclus, uniquement s'ils n'ont aucun impact sur les sessions.

## Rapport pour IA (v0.2, sans réseau)

Après une analyse, le bouton « Rapport pour une IA » génère un fichier Markdown que l'utilisateur colle lui-même dans l'IA de son choix (ChatGPT, Claude…). **Kleaner n'envoie rien.**

Contenu du rapport :

1. **Un prompt** qui demande à l'IA de :
   - poser des questions courtes, un groupe à la fois, en commençant par les plus gros gains (« Utilises-tu encore ce jeu ? Ce projet ? »)
   - finir par une liste classée par espace gagné, avec la méthode pour chaque élément (onglet Nettoyage de Kleaner, désinstallation, Corbeille) et le risque
   - ne jamais conseiller de toucher aux fichiers système
2. **Le résumé de l'analyse** : total, répartition par type, 200 plus gros fichiers et 200 plus gros dossiers, avec taille et date de modification.
3. **Les estimations du catalogue** : espace récupérable par élément, et ce que ça change.
4. **Les programmes installés**, avec taille et date d'installation.

Règles :

- **Le nom d'utilisateur est masqué** : `C:\Users\<nom>` devient `%USERPROFILE%`.
- **Aperçu obligatoire** : le rapport s'affiche avant d'être copié ou enregistré, avec un avertissement : « Il contient des noms de fichiers et de dossiers. Relis-le avant de le partager. »
- Deux actions : « Copier » (presse-papiers) ou « Enregistrer » dans `%LOCALAPPDATA%\Kleaner\rapports\`, puis ouverture dans l'Explorateur. La génération est notée dans le journal.

## Évolution v2 (hors v1) : recommandations personnalisées par IA intégrées

Idée : une IA regarde le résultat de l'analyse et propose des actions adaptées à la personne. Par exemple : « 40 Go de projets non ouverts depuis 1 an », « 3 jeux jamais lancés », « installeurs déjà installés dans Téléchargements ».

Contraintes si on le fait :

1. **Désactivé par défaut**, activé explicitement par l'utilisateur. Le reste de Kleaner continue de fonctionner sans réseau.
2. **On n'envoie que des métadonnées** (noms de dossiers, tailles, dates, extensions), jamais le contenu des fichiers. L'écran montre **exactement** ce qui sera envoyé, avant l'envoi.
3. Clé API fournie par l'utilisateur. Pas de serveur Kleaner.
4. **L'IA propose, elle n'agit pas.** Ses suggestions passent par les mêmes garde-fous : Corbeille pour les fichiers personnels, catalogue pour les suppressions définitives, confirmation.

Avant l'IA, on ajoute en v1.x des **règles locales** sans réseau : vieux installeurs, `node_modules` inactifs, fichiers jamais ouverts depuis N mois. Elles couvrent probablement une bonne partie du besoin.

## Questions ouvertes

1. Logo et icône : à faire ?
2. IA v2 : modèle cloud (avec la clé API de l'utilisateur) ou modèle local ? Un modèle local garde « aucun réseau », mais il pèse plusieurs Go, ce qui va contre « léger ».

## Mode admin : lecture complète

En admin, l'analyse active le privilège de sauvegarde de Windows (`SeBackupPrivilege`) et relit en lecture seule (`FILE_FLAG_BACKUP_SEMANTICS`, `FILE_LIST_DIRECTORY`) les dossiers refusés : `System Volume Information`, `WindowsApps`, dossiers réservés à SYSTEM. Ce privilège ne sert qu'à lister ; aucune écriture. Un badge « Admin activé » est visible en permanence.

## Photos & vidéos, tri et graphiques (v0.2)

- **Onglet Photos & vidéos** : toutes les vidéos (≥ 1 Mo) et photos (≥ 50 Ko) trouvées par la dernière analyse, hors `C:\Windows`, Program Files, ProgramData, AppData, Corbeille et System Volume Information. Recherche, tri, sélection, envoi à la Corbeille (même garde que l'onglet Analyse), ouverture dans l'Explorateur. Affichage par pages de 300.
- **Tri partout** : taille, nom, date (et dossier ou éditeur selon la liste). Un second clic inverse le sens.
- **Graphiques** : anneau de répartition par type (Analyse) et d'espace récupérable par type de nettoyage (Nettoyage), barres des 10 plus gros dossiers. Couleurs catégorielles fixées par entité, palette validée (daltonisme, contraste) en clair et sombre, légende toujours accompagnée de la valeur et du pourcentage.
- **Visionneuse** : « Voir » (photo, clic pour 100 %) et « Lire » (vidéo) dans Kleaner, ← → pour naviguer. Le protocole `asset` de Tauri est actif avec une portée **vide** : chaque fichier n'est autorisé qu'au moment où on l'ouvre, et seulement si c'est une photo ou une vidéo locale (`check_media`). Formats non lisibles par WebView2 (MKV, AVI, HEIC, RAW…) : bouton « Ouvrir avec le lecteur par défaut » (ShellExecute, mêmes vérifications).

## Révision 3 (retours de test)

- **English first** : l'interface démarre en anglais ; le français se choisit dans Settings. README en anglais d'abord. Messages d'erreur Rust en anglais.
- **Catalogue général** (42 entrées) : ajout Vivaldi, Chromium, Steam, Epic Games, Adobe (cache média), Maven, Go, Bun, Composer, Dart/Flutter, restes d'installeurs NVIDIA/AMD. Les éléments vides ou non installés sont masqués (case « afficher aussi les éléments vides »).
- **Tri par en-têtes de colonnes** (clic = tri, re-clic = sens inverse) dans Analyse, Photos & vidéos et Programmes.
- **Programmes** : colonnes « taille mesurée » et « taille déclarée », chacune masquable.
- **Photos & vidéos** : affichage Liste (défaut) ou Grille avec miniatures chargées à l'affichage ; choix mémorisé.
- **Nettoyage** : indicateur « Estimating… » pendant le rafraîchissement.

## Révision 4

- **Base de connaissances** (`knowledge.json`, ~95 règles) : verdict par fichier et dossier — fiches précises → catalogue → programme propriétaire → règles générales (type, emplacement) → inconnu. Kleaner ne devine jamais. Fichiers critiques (casse, système, à désinstaller) : avertissement + case « j'ai compris » avant la Corbeille (on avertit, on n'empêche pas).
- **Choix des disques** : internes cochés, USB et réseau au choix. `start_scan` n'accepte que des racines de disques présents.
- **Espace libre** par disque (stats et anneau). **Dernier lancement** des programmes via Prefetch (admin).
- **Instance unique** : relancer l'exe ramène la fenêtre existante ; la relance admin attend la fermeture de l'ancienne.
- **Thème** : Comme Windows (défaut, suit en direct) / Clair / Sombre.
- **À propos** : ce que fait / ne fait pas Kleaner, crédits swaks, liens GitHub. Les liens ouvrent le navigateur ; Kleaner lui-même ne se connecte toujours à rien.
- README anglais (`README.md`) et français (`README.fr.md`) séparés.

## Révision 5

- **Doublons** : contenu personnel uniquement (photos, vidéos, musique, documents, archives, installeurs ≥ 1 Mo, hors dossiers de programmes et `node_modules`). Taille → échantillon → contenu complet (blake3). Une copie reste toujours ; envoi à la Corbeille.
- **Vieux projets** : dossiers régénérables (`node_modules`, `.venv`, `target`, `.next`, `build`, `.gradle`, `.dart_tool`, `obj/bin`) reconnus par leur nom + le fichier du projet à côté (`package.json`, `Cargo.toml`…), hors Electron (`resources\app`). Date = dernière modification du projet hors ces dossiers. Suppression définitive, seulement des dossiers trouvés par la dernière analyse.
- **Carte du disque** : treemap des dossiers ≥ 64 Mo.
- **Simulation** : « Voir la liste exacte » avant nettoyage, liste complète en fichier texte.
- **Signature** : SignPath (gratuit open source), activée par secret ; voir `docs/signing.md`.
- **Distribution** : bucket Scoop dans le dépôt, manifestes winget générés et validés ; voir `docs/distribution.md`. README généré par `scripts/gen-readme.cjs`.

## Restes de programmes désinstallés

Dans l'onglet Programmes. Candidats : sous-dossiers d'`AppData\Local`, `AppData\Roaming`, `AppData\LocalLow`, `AppData\Local\Programs`, dossiers cachés du profil liés à une appli précise (`.vscode`, `.cursor`, `.eclipse`…), et — signalés seulement — `Program Files` et `ProgramData`. Un dossier est proposé s'il n'est rattaché à aucun programme installé (nom, éditeur, dossier d'installation), ni à une règle précise ou au catalogue, ni à un dossier partagé (Microsoft, pilotes, Android, WSL…), pèse ≥ 10 Mo et n'a pas changé depuis 30 jours. Verdict « reste probable » : envoi à la Corbeille (restaurable), seulement des dossiers trouvés par la dernière recherche. Les racines d'AppData sont protégées par la garde.

## Révision 6

- **Accueil** (page par défaut) : espace libre des disques, gros bouton « Analyser », puis recommandations chiffrées (nettoyage sans risque, restes, vieux projets, gros fichiers, doublons, OneDrive).
- **Restaurer** depuis le Journal ce que Kleaner a envoyé à la Corbeille.
- **Rapport de bug** : fichier local (version, Windows, admin, actions et erreurs, sans chemins) + ticket GitHub pré-rempli sans données ; rien n'est envoyé par Kleaner.
- **OneDrive** : gros fichiers synchronisés gardés en local → « Libérer de l'espace » (attributs +Unpinned -Pinned, action officielle de Windows). Rien n'est supprimé.
- **Mise à jour** : vérification optionnelle, **désactivée par défaut**, qui lit la liste publique des releases GitHub (`api.github.com`, autorisé dans la CSP). Seule exception à « aucun réseau », annoncée dans l'avertissement et À propos.

## Révision 7

- **Cases bloquées dans Analyse** : un fichier rangé dans un dossier qui ne se supprime qu'en entier (`.venv`, `node_modules`, `.git`, toolchain rustup, env conda, extension VS Code, navigateurs headless, paquets npm globaux : champ `whole` des règles) a le verdict « casse » et n'est pas cochable. Sa fiche, ouverte au clic sur la ligne grisée, renvoie au dossier entier.
- **Programmes** : une entrée de désinstallation périmée laissée par une mise à jour (même désinstalleur, même dossier, ex. Opera GX) est fusionnée ; on garde la version la plus récente. La carte « Restes de programmes » est au-dessus de la liste.
- **Restes** : aussi les dossiers d'anciennes versions (`120.0.5543.85`…) dans `AppData\Local\Programs\<appli>`, sauf la version installée et la plus récente, avec les mêmes seuils (≥ 10 Mo, rien de modifié depuis 30 jours).
- **Vieux projets** : un projet modifié ces 30 derniers jours porte « Utilisé récemment » et n'est jamais coché d'office.
- **Vider la Corbeille** : proposé juste après chaque envoi à la Corbeille (Analyse, restes, doublons, photos et vidéos), avec une confirmation qui dit que la restauration ne sera plus possible.

## Feuille de route : au-delà du nettoyage (v0.3+)

Kleaner devient un outil d'entretien du PC, avec les mêmes règles : il montre, il explique, il ne devine pas, et chaque action est réversible ou confirmée. Ordre prévu :

1. **Démarrage** : tout ce qui se lance à l'ouverture de session (clés `Run` HKCU et HKLM, dossiers Démarrage, tâches planifiées « à l'ouverture de session », services tiers en démarrage automatique), avec éditeur, emplacement et fiche. Durée réelle du démarrage lue dans le journal `Microsoft-Windows-Diagnostics-Performance` (événement 100, admin). Désactiver passe par `StartupApproved`, le même mécanisme que le Gestionnaire des tâches : réversible en un clic, rien n'est supprimé. Les entrées Microsoft et pilotes sont marquées « à garder ».
2. **Raccourcis cassés** : `.lnk` du Bureau, du menu Démarrer (utilisateur et commun) et de la barre des tâches dont la cible n'existe plus (ni les cibles réseau ou sur disque débranché : signalées « disque absent », pas cassées). Envoi à la Corbeille.
3. **Santé**, en lecture seule :
   - disques : état SMART, usure et température des SSD (`MSFT_PhysicalDisk`, `MSFT_StorageReliabilityCounter`) ;
   - batterie : capacité restante par rapport à la capacité d'origine ;
   - plantages d'applis et écrans bleus des 30 derniers jours (journal Application et `WER`) ;
   - redémarrage en attente, intégrité du système (`DISM /Online /Cleanup-Image /CheckHealth`, sans réparation automatique) ;
   - mémoire saturée, disque presque plein, mode d'alimentation.
4. **Sécurité**, en lecture seule : Defender actif et définitions récentes, pare-feu, UAC, SmartScreen, Secure Boot, TPM, BitLocker, mises à jour en retard et fin de support. Chaque point a un bouton vers l'écran Windows qui le règle. Kleaner n'est pas un antivirus et ne change aucun réglage de sécurité lui-même.
5. **Entrées de programmes fantômes** : entrées « Programmes » dont le dossier et le désinstalleur ont disparu. Suppression de la clé seulement après sauvegarde `.reg` dans `%LOCALAPPDATA%\Kleaner\backups` et confirmation, restaurable depuis le Journal.
6. **Cloud** : dossiers locaux de Google Drive et Dropbox (cache, fichiers gardés hors ligne), sur le modèle d'OneDrive. Rien en ligne : pas de connexion aux comptes.

### Ce que Kleaner ne fera pas

- **« Nettoyer » ou « réparer » le registre** en général : Microsoft ne le prend pas en charge, aucun gain mesurable, risque de casser des programmes. Seul le cas vérifiable du point 5 est traité.
- **« Boosts » et réglages miracles** (désactiver des services Windows, vider la RAM, « optimiser » le réseau) : placebo ou nuisible. Windows défragmente et fait le TRIM seul. Kleaner donne des recommandations vérifiables à la place.
- **Antivirus maison** ou modification des réglages de sécurité.
- **Connexion aux comptes cloud** : contraire à « aucun réseau ».

## Révision 8 : feuille de route réalisée (T18 à T22)

Écrite après l'implémentation ; elle précise ou corrige ce que la feuille de route annonçait.

- **Exception à « jamais d'écriture dans le registre »**, seulement sur demande et réversible : (1) `StartupApproved` pour activer ou désactiver un programme au démarrage ; (2) la suppression d'une entrée fantôme de la liste des programmes, après sauvegarde `.reg`. Aucun autre nettoyage ou « réparation » du registre.
- **Navigation** : « Démarrage » et « Santé et sécurité » rejoignent l'essentiel ; « Raccourcis cassés » est dans « Plus d'outils ». « Mises à jour et santé » devient « Mises à jour et pilotes » (la santé a son onglet). « OneDrive » devient « Cloud ».
- **Démarrage** (`startup.rs`) : clés `Run` (HKCU, HKLM, HKLM 32 bits), dossiers Démarrage (utilisateur et commun), tâches planifiées déclenchées à l'ouverture de session ou au démarrage (hors `\Microsoft\`), services automatiques hors `Windows` et Defender. Désactiver : `StartupApproved` (12 octets, comme le Gestionnaire des tâches) ou `Disable-ScheduledTask` ; les éléments « pour tous les utilisateurs » demandent l'admin. **Les services sont en lecture seule** (les arrêter casse trop de choses) : écart par rapport à la feuille de route. Les éléments Windows, sécurité et pilotes portent « À garder » et demandent une confirmation avant d'être désactivés. Durées de démarrage (événement 100) et applications lentes (événement 101) : journal `Diagnostics-Performance`, lisible en admin seulement. Le changement n'est accepté que pour un élément de la dernière liste.
- **Raccourcis cassés** (`shortcuts.rs`) : `.lnk` du Bureau (utilisateur, public, OneDrive), du menu Démarrer et de la barre des tâches, hors dossier `Tombstones` (trace des épingles retirées). Cassé = cible locale absolue, sur un disque fixe ou amovible présent, introuvable. Ignorés : réseau, disque absent, Store (`WindowsApps`), raccourci sans chemin. Corbeille via la même garde que les fichiers personnels ; les raccourcis communs à tous demandent l'admin.
- **Santé et sécurité** (`health.rs`, lecture seule) : disques (état, usure et température SSD en admin), espace libre, batterie, écrans bleus et plantages d'applis sur 30 jours, redémarrage en attente, durée depuis le dernier redémarrage, mémoire, mode d'alimentation, fin de support de Windows, dernière mise à jour, antivirus (Centre de sécurité), pare-feu, UAC, SmartScreen, Secure Boot, TPM (admin), BitLocker. Contrôle d'intégrité `Repair-WindowsImage -CheckHealth` sur un bouton séparé, admin seulement, sans réparation. Chaque alerte ouvre un écran de Windows pris dans la liste fermée `health::PAGES` : l'interface n'envoie jamais d'adresse libre. Les textes viennent de Rust en anglais et en français.
- **Entrées fantômes** (`programs::ghosts`) : entrée « Programmes » dont le désinstalleur (chemin absolu, disque présent, sans variable d'environnement) est introuvable **et** dont le dossier d'installation déclaré est absent ou introuvable. Cas douteux : jamais proposé. Suppression : export `reg export` dans `%LOCALAPPDATA%\Kleaner\backups\ghost-*.reg`, puis suppression de la clé ; si la sauvegarde échoue, rien n'est supprimé. Restauration depuis le Journal (`reg import`, seulement un fichier `ghost-*.reg` de ce dossier).
- **Cloud** (`onedrive.rs`) : OneDrive et Dropbox (dossiers lus dans `info.json`, compte personnel gratuit Basic exclu : pas de synchronisation intelligente). L'action du journal devient `cloud-free-up`. **Google Drive** : en mode diffusion il utilise son propre disque virtuel, qu'on ne peut pas libérer de l'extérieur ; Kleaner explique seulement la marche à suivre (règle `google-drive-cache`, verdict « casse »). Règle `dropbox-cache` pour `.dropbox.cache`.
- **Règles de connaissance** : champ `whole` (dossier supprimable seulement en entier), voir la révision 7.

## Révision 9 : catalogue communautaire (Winapp2)

- **Source** : Winapp2.ini (projet Winapp2, CC BY-SA 4.0, 4 068 entrées). Converti à l'avance par `src-tauri/src/winapp2.rs` en `src-tauri/winapp2.json` (environ 1 100 nettoyages), embarqué dans l'exe : aucun réseau. Attribution et licence : `WINAPP2-NOTICE.md`, README, page À propos. Seul `winapp2.json` suit la licence CC BY-SA ; le reste reste sous MIT.
- **Ce qui est gardé** : dossiers dont le nom dit qu'ils sont jetables (cache, temp, logs, crash, dump, thumbnails, shader), ou motifs `*.log`, `*.tmp`, `*.dmp`, `*.etl` seulement, dans AppData, LocalLow, dossiers cachés du profil, ProgramData (admin). Racines et variables identiques à celles du catalogue intégré, au moins deux niveaux de profondeur.
- **Ce qui est écarté** : tout le registre (`RegKey`), les entrées avec `Default=False`, avertissement ou exclusions, les mots interdits (cookies, historique, mots de passe, sessions, sauvegardes, téléchargements, documents, logiciels de sécurité, DriveFS, hors ligne, installeurs…), les dossiers Microsoft et Store d'AppData, et ce que le catalogue intégré couvre déjà (par nom de programme et par dossier).
- **Pas de sous-dossiers sans RECURSE** (`flat`) : respecte Winapp2 et évite de toucher aux fichiers `.log` de LevelDB (stockage local) dans un profil de navigateur. Champ `flat` ajouté au moteur du catalogue.
- **Défense en profondeur** : le même filtre (`entry_ok`) est appliqué au chargement ; un test vérifie que le fichier livré est accepté en entier. Les tests du catalogue (chemins étroits, variables autorisées, jamais de sessions ni de logins) s'appliquent aussi aux entrées communautaires.
- **Interface** : groupe « Catalogue communautaire » dans Nettoyage. Jamais coché en bloc (ni par « Cocher les éléments sans risque », ni dans l'espace libérable de l'Accueil). Chaque ligne montre le dossier vidé ; le texte explicatif, identique partout, est donné une fois en tête.
- **Mise à jour** : voir `WINAPP2-NOTICE.md` (test `winapp2::tests::regenerate`).
- **Signature de code** : SignPath a répondu qu'il faut plus d'étoiles et de visibilité. La mention « signature gratuite SignPath » est retirée du README ; l'exe reste non signé (SmartScreen). On redemandera plus tard.

## Révision 10 : visibilité, niche, scan rapide, règles proposées

- **README refait** (`scripts/gen-readme.cjs`) : une phrase, la capture des verdicts, pourquoi Kleaner (il explique, il est prudent, il est discret, il connaît les PC de développeurs, de joueurs et d'IA), liste courte de ce qu'il fait, téléchargement, ce qu'il ne fait jamais, comment aider. Le tableau du catalogue est déplacé dans `docs/catalog.md` (et `.fr.md`). La mention de signature SignPath est retirée (refus pour l'instant).
- **Captures d'écran** : mode `demo` de Vite (`src/demo/`, `npm run demo:build && npm run demo:shots`) qui remplace Tauri par de fausses données ; les verdicts viennent des vraies règles de `knowledge.json`. Chrome sans fenêtre prend les images dans `docs/img/`. Absent de l'exe.
- **Niche développeurs, joueurs, IA** : nouveaux nettoyages `dev-uv`, `dev-poetry`, `dev-cypress` ; nouvelles fiches GPT4All, Jan, pnpm, configuration Docker, éditeurs Unity, boxes Vagrant. Le catalogue communautaire (Winapp2) couvre le reste.
- **Règles proposées par les utilisateurs** : sur un verdict « inconnu », le bouton « Proposer une règle » ouvre le formulaire GitHub `rule-proposal.yml`, pré-rempli avec le chemin sans nom d'utilisateur. Kleaner n'envoie rien ; l'utilisateur relit et valide dans son navigateur. Format des règles : `docs/rules.md`.
- **Scan rapide NTFS** (`mft.rs`) : en admin, la table des fichiers du volume (MFT) est lue en lecture seule, d'un bloc, au lieu d'ouvrir les dossiers un par un. L'index alimente le même parcours (`scan::walk`) : mêmes règles, mêmes listes. Tout échec ou doute (pas admin, pas NTFS, MFT décrite par des enregistrements d'extension, fichiers qui ne représentent pas au moins 60 % de l'espace utilisé) retombe sur le parcours classique ; `KLEANER_NO_MFT=1` le désactive. Les fichiers système (enregistrements 0 à 23) sont masqués comme dans le listage normal. Un badge « Scan NTFS rapide » apparaît dans les résultats.
  - **Mesuré sur le vrai disque C: (420 Go, 1,2 million de fichiers)** : 2 s au lieu de 8 à 21 s (4 à 10 fois plus vite selon le cache de Windows).
  - **Écart avec le scan classique** : 3 % de l'espace et 5 % des fichiers en moins. Ce n'est pas une perte : le scan classique compte deux fois les dossiers virtuels des applis du Store (MSIX), par exemple le dossier Claude d'AppData Roaming (11,5 Go) et le magasin pnpm : une fois dans Packages, une fois via le recouvrement du système. Le scan rapide montre le disque réel (le chemin physique sous Packages). Les noms de liens physiques stockés dans des enregistrements d'extension (WinSxS) sont lus.
  - Vérifier : en terminal administrateur, `cargo test --manifest-path src-tauri/Cargo.toml --release mft::tests::real -- --ignored --nocapture` compare les deux scans (tolérance 8 %).
