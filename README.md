# Kleaner

**FR** · [EN](#english)

Nettoyeur Windows gratuit, open source et **transparent**. Kleaner analyse tous tes disques, fichiers cachés compris, et supprime ce qu'on ne voit pas d'habitude : temporaires, caches, restes de mises à jour, dumps… **sans rien changer à ton utilisation du PC** : tu restes connecté partout, tu ne perds ni fichiers ni réglages.

## Ce que fait Kleaner

- **Analyse** tous les disques fixes et montre les plus gros fichiers et dossiers, et la répartition par type.
- **Nettoie** les éléments du [catalogue](#catalogue) : chaque élément explique ce qu'il supprime, pourquoi c'est inutile et ce que ça change.
- **Envoie à la Corbeille** les fichiers personnels que tu choisis (jamais de suppression directe).
- **Désinstalle** des programmes en lançant leur désinstalleur officiel.
- **Journalise** chaque action dans `%LOCALAPPDATA%\Kleaner\journal.log`.

## Ce que Kleaner ne fait jamais

- Agir sans ta confirmation.
- Toucher à tes mots de passe, cookies, sessions ou historique.
- Écrire dans le registre.
- Se connecter à Internet : **aucune télémétrie, aucune mise à jour automatique**.
- Demander les droits admin sans que tu cliques sur « Analyse complète (admin) ».
- Supprimer définitivement un fichier en dehors des dossiers listés dans [`src-tauri/cleaners.json`](src-tauri/cleaners.json). La vérification est faite dans le code Rust ([`guard.rs`](src-tauri/src/guard.rs)) et couverte par des tests.

## Télécharger

1. Va dans [Releases](https://github.com/Swaksm/Kleaner/releases) et télécharge `kleaner.exe` et `kleaner.exe.sha256`.
2. Vérifie l'empreinte (PowerShell) :
   ```powershell
   (Get-FileHash kleaner.exe -Algorithm SHA256).Hash.ToLower()
   ```
   Le résultat doit être identique au contenu de `kleaner.exe.sha256`.
3. Lance `kleaner.exe`. Rien à installer : c'est un exe portable.

**Avertissement Windows (SmartScreen).** L'exe n'est pas signé (un certificat coûte cher pour un projet gratuit). Windows affichera « Windows a protégé votre ordinateur » : clique sur *Informations complémentaires* puis *Exécuter quand même*. Certains antivirus peuvent aussi réagir, car Kleaner supprime des fichiers et peut demander les droits admin. Le code est entièrement lisible ici, et l'exe est compilé par GitHub Actions à partir de ce code.

Windows 10 ou 11 64 bits requis (WebView2, présent par défaut sur Windows 11 et Windows 10 à jour).

## Retirer Kleaner

*Paramètres → Retirer Kleaner* efface `%LOCALAPPDATA%\Kleaner` et `%LOCALAPPDATA%\com.swaksm.kleaner`. Supprime ensuite `kleaner.exe`. Rien d'autre n'a été installé.

## Catalogue

Tout ce que Kleaner peut supprimer définitivement. « Admin » : nécessite l'analyse complète.

| Élément | Supprime | Ce que ça change | Admin |
|---|---|---|---|
| Fichiers temporaires | Fichiers laissés dans %TEMP% par les installeurs et les applis. | Rien. |  |
| Cache des miniatures | Aperçus d'images et d'icônes mémorisés par l'Explorateur. | Miniatures un peu plus lentes à apparaître la première fois. |  |
| Cache Internet de Windows | Copies de pages et fichiers web gardées par Windows et les anciennes applis. | Rien. |  |
| Rapports de plantage | Copies de la mémoire des applis qui ont planté, et rapports d'erreurs Windows. | Rien. |  |
| Caches de la carte graphique | Shaders compilés par DirectX et les pilotes NVIDIA ou AMD. | Premier lancement des jeux un peu plus long. |  |
| Corbeille | Tous les fichiers actuellement dans la Corbeille, sur tous les disques. | Irréversible : ces fichiers ne pourront plus être restaurés. |  |
| Google Chrome : cache | Copies locales des pages, scripts et shaders déjà chargés. | Tu restes connecté. Sites un peu plus lents au premier chargement. |  |
| Microsoft Edge : cache | Copies locales des pages, scripts et shaders déjà chargés. | Tu restes connecté. Sites un peu plus lents au premier chargement. |  |
| Brave : cache | Copies locales des pages, scripts et shaders déjà chargés. | Tu restes connecté. Sites un peu plus lents au premier chargement. |  |
| Opera : cache | Copies locales des pages et scripts déjà chargés. | Tu restes connecté. Sites un peu plus lents au premier chargement. |  |
| Firefox : cache | Copies locales des pages et fichiers déjà chargés (dossier cache2). | Tu restes connecté. Sites un peu plus lents au premier chargement. |  |
| Discord : cache | Images, vidéos et scripts déjà affichés dans Discord. | Tu restes connecté. Images un peu plus lentes à apparaître. |  |
| Slack : cache | Images, fichiers et scripts déjà affichés dans Slack. | Tu restes connecté. Images un peu plus lentes à apparaître. |  |
| Microsoft Teams : cache | Images, fichiers et scripts déjà affichés dans Teams (ancienne et nouvelle version). | Tu restes connecté. Premier lancement un peu plus long. |  |
| VS Code : cache | Scripts précompilés et paquets d'extensions déjà installés. | Premier lancement un peu plus long. |  |
| npm : cache | Paquets npm déjà téléchargés, gardés en copie. | Le prochain install ou build retélécharge ce qui manque. |  |
| Yarn : cache | Paquets Yarn déjà téléchargés, gardés en copie. | Le prochain install ou build retélécharge ce qui manque. |  |
| pip : cache | Paquets Python déjà téléchargés par pip. | Le prochain install ou build retélécharge ce qui manque. |  |
| NuGet : paquets et cache | Paquets .NET téléchargés par NuGet. | Le prochain install ou build retélécharge ce qui manque. |  |
| Gradle : cache | Dépendances et résultats de build gardés par Gradle. | Le prochain install ou build retélécharge ce qui manque. |  |
| Cargo : cache | Crates Rust téléchargées et décompressées par Cargo. | Le prochain install ou build retélécharge ce qui manque. |  |
| Temporaires de Windows | Fichiers laissés dans C:\Windows\Temp par Windows et les installeurs. | Rien. | oui |
| Rapports de plantage système | Rapports d’erreurs Windows et petits dumps mémoire après un écran bleu. | Rien. | oui |
| Dump mémoire complet | Le fichier C:\Windows\MEMORY.DMP créé après un écran bleu. | Rien. | oui |
| Cache d’optimisation de livraison | Morceaux de mises à jour Windows gardés pour les partager avec d’autres PC. | Rien. | oui |
| Téléchargements Windows Update | Fichiers de mises à jour déjà téléchargés dans SoftwareDistribution\Download. | Une mise à jour en attente sera retéléchargée. | oui |
| Anciens journaux de Windows | Anciens journaux d’installation des composants Windows (CbsPersist) de plus de 7 jours. | Rien. | oui |
| Ancienne version de Windows | Les dossiers Windows.old, $Windows.~BT et $Windows.~WS laissés après une mise à jour majeure. | Irréversible : tu ne pourras plus revenir à la version précédente de Windows. | oui |
| Anciens composants Windows (WinSxS) | Anciennes versions de composants remplacées par des mises à jour. | Les mises à jour déjà installées ne pourront plus être désinstallées. | oui |
| Hibernation | Le fichier hiberfil.sys, de la taille d’une bonne partie de la RAM. | Hibernation et démarrage rapide désactivés. Réactivable avec powercfg /hibernate on. | oui |
| Anciens points de restauration | Tous les points de restauration système sauf le plus récent. | Tu ne pourras plus revenir à un état plus ancien que le dernier point. | oui |

Les caches de navigateurs et d'applis ne ciblent que les dossiers `Cache`, `Code Cache`, `GPUCache` et `cache2`. Un test fait échouer la compilation si un chemin du catalogue touche `Cookies`, `Local Storage`, `IndexedDB`, `Login Data`, `Session Storage`, `Network` ou `Service Worker`.

## Développement

```bash
npm install
npm run tauri dev
npm test
npm run lint
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri build -- --no-bundle
```

Prérequis : Node 22+, Rust stable, Visual Studio Build Tools (C++). Spécification : [SPEC.md](SPEC.md). Avant chaque release : [docs/release-checklist.md](docs/release-checklist.md).

Licence [MIT](LICENSE), sans garantie.

---

## English

Free, open-source and **transparent** Windows cleaner. Kleaner scans all your drives, hidden files included, and removes what you don't usually see: temporary files, caches, update leftovers, dumps… **without changing how you use your PC**: you stay logged in everywhere, and lose no files or settings.

### What Kleaner does

- **Scans** all fixed drives and shows the largest files and folders, and the breakdown by type.
- **Cleans** the items of the [catalog](#catalog): each item explains what it removes, why it is useless and what changes.
- **Sends to the Recycle Bin** the personal files you choose (never a direct deletion).
- **Uninstalls** programs by launching their official uninstaller.
- **Logs** every action to `%LOCALAPPDATA%\Kleaner\journal.log`.

### What Kleaner never does

- Act without your confirmation.
- Touch your passwords, cookies, sessions or history.
- Write to the registry.
- Connect to the Internet: **no telemetry, no automatic updates**.
- Ask for admin rights unless you click "Full scan (admin)".
- Permanently delete a file outside the folders listed in [`src-tauri/cleaners.json`](src-tauri/cleaners.json). This is enforced in Rust ([`guard.rs`](src-tauri/src/guard.rs)) and covered by tests.

### Download

1. Go to [Releases](https://github.com/Swaksm/Kleaner/releases) and download `kleaner.exe` and `kleaner.exe.sha256`.
2. Check the hash (PowerShell):
   ```powershell
   (Get-FileHash kleaner.exe -Algorithm SHA256).Hash.ToLower()
   ```
   It must match the content of `kleaner.exe.sha256`.
3. Run `kleaner.exe`. Nothing to install: it is a portable exe.

**Windows warning (SmartScreen).** The exe is not signed (a certificate is expensive for a free project). Windows will show "Windows protected your PC": click *More info* then *Run anyway*. Some antivirus tools may also react, because Kleaner deletes files and can ask for admin rights. All the code is readable here, and the exe is built by GitHub Actions from this code.

Requires 64-bit Windows 10 or 11 (WebView2, included in Windows 11 and up-to-date Windows 10).

### Removing Kleaner

*Settings → Remove Kleaner* erases `%LOCALAPPDATA%\Kleaner` and `%LOCALAPPDATA%\com.swaksm.kleaner`. Then delete `kleaner.exe`. Nothing else was installed.

### Catalog

Everything Kleaner can permanently delete. "Admin": requires the full scan.

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

Browser and app caches only target the `Cache`, `Code Cache`, `GPUCache` and `cache2` folders. A test fails the build if a catalog path touches `Cookies`, `Local Storage`, `IndexedDB`, `Login Data`, `Session Storage`, `Network` or `Service Worker`.

MIT [license](LICENSE), no warranty.
