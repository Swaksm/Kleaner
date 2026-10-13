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
- **Jamais** : supprimer hors du catalogue, toucher aux cookies, sessions ou identifiants, écrire dans le registre, s'élever sans clic de l'utilisateur, envoyer de la télémétrie, créer des fichiers hors de `%LOCALAPPDATA%\Kleaner`.

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
