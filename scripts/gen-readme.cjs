// Génère README.md (anglais), README.fr.md (français), docs/catalog.md et docs/catalog.fr.md.
// Le catalogue vient de cleaners.json ; les nombres, de knowledge.json et winapp2.json.
// Usage : node scripts/gen-readme.cjs .
const fs = require("fs");
const root = process.argv[2];
const readJson = (p) => JSON.parse(fs.readFileSync(`${root}/${p}`, "utf8"));
const cat = readJson("src-tauri/cleaners.json");
const rules = readJson("src-tauri/knowledge.json").length;
const community = readJson("src-tauri/winapp2.json").entries.length;
const roundDown = (n) => Math.floor(n / 100) * 100;
const B = "`";
const code = (s) => B + s + B;
const fence = (lang, body) => [B + B + B + lang, ...body, B + B + B].join("\n");
const cell = (s) => s.replace(/\|/g, "\\|");
const table = (l, head, yes) =>
  [
    "| " + head.join(" | ") + " |",
    "|---|---|---|---|",
    ...cat.map((e) => "| " + [cell(e.label[l]), cell(e.what[l]), cell(e.effect[l]), e.admin ? yes : ""].join(" | ") + " |"),
  ].join("\n");

const hash = fence("powershell", ["(Get-FileHash kleaner.exe -Algorithm SHA256).Hash.ToLower()"]);
const scoop = fence("powershell", ["scoop bucket add kleaner https://github.com/Swaksm/Kleaner", "scoop install kleaner/kleaner"]);
const dev = fence("bash", [
  "npm install",
  "npm run tauri dev",
  "npm test",
  "npm run lint",
  "cargo test --manifest-path src-tauri/Cargo.toml",
  "cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings",
  "npm run tauri build -- --no-bundle",
]);
const shots = fence("bash", ["npm run demo:build", "npm run demo:shots"]);

const badges =
  "[![Release](https://img.shields.io/github/v/release/Swaksm/Kleaner?include_prereleases&label=release)](https://github.com/Swaksm/Kleaner/releases) " +
  "[![License: MIT](https://img.shields.io/github/license/Swaksm/Kleaner)](LICENSE) " +
  "[![Downloads](https://img.shields.io/github/downloads/Swaksm/Kleaner/total)](https://github.com/Swaksm/Kleaner/releases)";

const gallery = (alt) =>
  [
    "<p>",
    `  <img src="docs/img/home.png" alt="${alt.home}" width="49%"> <img src="docs/img/cleanup.png" alt="${alt.cleanup}" width="49%">`,
    `  <img src="docs/img/startup.png" alt="${alt.startup}" width="49%"> <img src="docs/img/health.png" alt="${alt.health}" width="49%">`,
    "</p>",
  ].join("\n");

const guard = "[" + code("guard.rs") + "](src-tauri/src/guard.rs)";
const cleanersLink = "[" + code("cleaners.json") + "](src-tauri/cleaners.json)";
const winapp2Link = "[" + code("winapp2.json") + "](src-tauri/winapp2.json)";
const knowledgeLink = "[" + code("knowledge.json") + "](src-tauri/knowledge.json)";

const en = [
  "<img src=\"docs/logo.svg\" alt=\"Kleaner\" width=\"96\">",
  "",
  "# Kleaner",
  "",
  "**English** · [Français](README.fr.md)",
  "",
  badges,
  "",
  "**A free Windows cleaner that tells you what you can delete, and what breaks if you do.**",
  "",
  "![Every big file and folder gets a verdict](docs/img/verdicts.png)",
  "",
  "Kleaner scans your drives, explains what takes up the space, and cleans what is safe: caches, temporary files, leftovers of uninstalled programs, old project folders. You stay logged in everywhere and lose no files or settings. Free, open source (MIT), no account, no ads, no telemetry.",
  "",
  "## Why Kleaner",
  "",
  "- **It explains.** Every large file and folder gets a verdict (safe, clean with its tool, uninstall, breaks something, your files, system, unknown) with what it is, what happens if you delete it and how to free the space properly. " + rules + " hand-written rules in " + knowledgeLink + "; with no rule, the verdict is « unknown »: Kleaner never guesses.",
  "- **It is careful.** Your files only go to the Recycle Bin, and the Log can restore them. Permanent deletion is limited to a public list of caches (" + cleanersLink + " and the filtered community catalog " + winapp2Link + "), enforced in Rust (" + guard + ") and covered by tests. Nothing is ticked in advance, and « Preview the exact list » shows every file before anything is deleted.",
  "- **It is private.** No network (only an optional update check, off by default), no telemetry, no account. It never touches your passwords, cookies, sessions or history.",
  "- **It knows developer, gamer and AI machines.** " + code("node_modules") + ", " + code(".venv") + ", " + code("target") + ", " + code(".gradle") + " in projects untouched for months (with the command that rebuilds them), Docker and WSL disks, Android emulators, Ollama, LM Studio and Hugging Face models, PyTorch and conda caches, game launchers (Steam, Epic, Riot, EA, Ubisoft, GOG, Battle.net) and Minecraft modpack instances. On the author's own PC: 25 GB of old project folders and 8 GB left by an uninstalled Arduino.",
  "",
  gallery({ home: "Home: your drives and what you can free", cleanup: "Cleanup: nothing ticked in advance", startup: "Startup: what starts with Windows", health: "Health & security: a read-only check-up" }),
  "",
  "## What it does",
  "",
  "- **Scan** the drives you choose: free space, largest files and folders, breakdown by type, a disk map. Sort by size, name or date. In administrator mode on NTFS, Kleaner reads the file table directly (read-only): about 4 to 10 times faster (2 s instead of 8 to 21 s on a 420 GB system disk).",
  "- **Cleanup**: " + cat.length + " built-in items plus about " + roundDown(community) + " from the community catalog, each saying what it removes and what changes for you. [Full list](docs/catalog.md).",
  "- **Old projects**, **duplicates**, **photos & videos** (with a built-in viewer): free space in the places only you can judge, with a safety net.",
  "- **Programs**: uninstall with the official uninstaller, real size on disk, last launch date; find leftovers of uninstalled programs and dead entries in the list.",
  "- **Startup**: what starts with Windows and how long the last boots took. Turning an item off deletes nothing and can be undone.",
  "- **Health & security** (read-only): disks, battery, crashes, pending restart, antivirus, firewall, Secure Boot, BitLocker… Each alert opens the Windows screen that fixes it. Not an antivirus.",
  "- **Broken shortcuts**, **Cloud** (OneDrive and Dropbox: keep large files online only), **Updates & drivers** (Windows Update and winget, on click).",
  "- **Journal** of every action, **AI report** (optional, offline: a Markdown summary you paste into the AI of your choice), 6 languages, light and dark themes.",
  "",
  "## Download",
  "",
  "1. Go to [Releases](https://github.com/Swaksm/Kleaner/releases) and download " + code("kleaner.exe") + " and " + code("kleaner.exe.sha256") + ".",
  "2. Check the hash (PowerShell). It must match the content of " + code("kleaner.exe.sha256") + ":",
  "",
  hash,
  "",
  "3. Run " + code("kleaner.exe") + ". Nothing to install: it is a portable exe. Requires 64-bit Windows 10 or 11.",
  "",
  "Or with Scoop:",
  "",
  scoop,
  "",
  "**Windows warning (SmartScreen).** The exe is not signed yet (a certificate is expensive for a free project, and code-signing programs for open source ask for more stars first). Windows shows \"Windows protected your PC\": click *More info* then *Run anyway*. Some antivirus tools may react because Kleaner deletes files and can ask for admin rights. All the code is readable here, and the exe is built by GitHub Actions from it: the hash above proves you got that build.",
  "",
  "## What Kleaner never does",
  "",
  "- Act without your confirmation.",
  "- Touch your passwords, cookies, sessions or history.",
  "- Clean or \"repair\" the registry. It writes there only on your request, and only two things: the on/off switch of a startup item, and the removal of a dead program entry (saved to a .reg file first, restorable from the Log).",
  "- Connect to the Internet: **no telemetry, no automatic updates**.",
  "- Ask for admin rights unless you click \"Full scan (admin)\".",
  "- Permanently delete anything outside the public cache list. Personal files only ever go to the Recycle Bin.",
  "",
  "## Help make it better",
  "",
  "- **Kleaner shows « unknown » for something you recognise?** Click its verdict, then *Propose a rule*: a GitHub form opens in your browser, pre-filled without your user name. You read it and decide whether to send it. See [docs/rules.md](docs/rules.md).",
  "- **Bug?** *About → Report a problem* prepares a report with no personal paths.",
  "- A cache that is missing from the catalog? Open an issue with the folder and a source showing it is safe to delete.",
  "",
  "## Removing Kleaner",
  "",
  "*Settings → Remove Kleaner* erases " + code("%LOCALAPPDATA%\\Kleaner") + " and " + code("%LOCALAPPDATA%\\com.swaksm.kleaner") + ". Then delete " + code("kleaner.exe") + ". Nothing else was installed.",
  "",
  "## Development",
  "",
  dev,
  "",
  "Requirements: Node 22+, stable Rust, Visual Studio Build Tools (C++). Specification: [SPEC.md](SPEC.md) (French). Before each release: [docs/release-checklist.md](docs/release-checklist.md). The screenshots above come from a demo mode with fake data (" + code("src/demo") + "):",
  "",
  shots,
  "",
  "## Credits and licenses",
  "",
  "Made by [swaks](https://github.com/Swaksm). MIT [license](LICENSE), no warranty.",
  "",
  "The community catalog (" + code("src-tauri/winapp2.json") + ") is derived from [Winapp2](https://github.com/MoscaDotTo/Winapp2) by the Winapp2 project, under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). See [WINAPP2-NOTICE.md](WINAPP2-NOTICE.md). Everything else is MIT.",
  "",
].join("\n");

const fr = [
  "<img src=\"docs/logo.svg\" alt=\"Kleaner\" width=\"96\">",
  "",
  "# Kleaner",
  "",
  "[English](README.md) · **Français**",
  "",
  badges,
  "",
  "**Un nettoyeur Windows gratuit qui te dit ce que tu peux supprimer, et ce qui casse si tu le fais.**",
  "",
  "![Chaque gros fichier et dossier a un verdict](docs/img/verdicts.png)",
  "",
  "Kleaner analyse tes disques, explique ce qui prend la place, et nettoie ce qui est sans risque : caches, fichiers temporaires, restes de programmes désinstallés, vieux dossiers de projets. Tu restes connecté partout et tu ne perds ni fichier ni réglage. Gratuit, open source (MIT), sans compte, sans pub, sans télémétrie.",
  "",
  "## Pourquoi Kleaner",
  "",
  "- **Il explique.** Chaque gros fichier et dossier a un verdict (supprimable, à nettoyer avec son outil, à désinstaller, casse quelque chose, tes fichiers, système, inconnu) avec ce que c'est, ce qui se passe si tu le supprimes et comment libérer la place proprement. " + rules + " règles écrites à la main dans " + knowledgeLink + " ; sans règle, le verdict est « inconnu » : Kleaner ne devine jamais.",
  "- **Il est prudent.** Tes fichiers vont seulement à la Corbeille, et le Journal peut les restaurer. La suppression définitive se limite à une liste publique de caches (" + cleanersLink + " et le catalogue communautaire filtré " + winapp2Link + "), vérifiée en Rust (" + guard + ") et couverte par des tests. Rien n'est coché d'avance, et « Voir la liste exacte » montre chaque fichier avant toute suppression.",
  "- **Il est discret.** Aucun réseau (sauf une vérification de mise à jour facultative, désactivée par défaut), aucune télémétrie, aucun compte. Il ne touche jamais à tes mots de passe, cookies, sessions ou historique.",
  "- **Il connaît les PC de développeurs, de joueurs et d'IA.** " + code("node_modules") + ", " + code(".venv") + ", " + code("target") + ", " + code(".gradle") + " des projets pas touchés depuis des mois (avec la commande qui les recrée), disques Docker et WSL, émulateurs Android, modèles Ollama, LM Studio et Hugging Face, caches PyTorch et conda, launchers de jeux (Steam, Epic, Riot, EA, Ubisoft, GOG, Battle.net) et instances de modpacks Minecraft. Sur le PC de l'auteur : 25 Go de vieux projets et 8 Go laissés par un Arduino désinstallé.",
  "",
  gallery({ home: "Accueil : tes disques et ce que tu peux libérer", cleanup: "Nettoyage : rien n'est coché d'avance", startup: "Démarrage : ce qui se lance avec Windows", health: "Santé et sécurité : un bilan en lecture seule" }),
  "",
  "## Ce que fait Kleaner",
  "",
  "- **Analyse** les disques que tu choisis : espace libre, plus gros fichiers et dossiers, répartition par type, carte du disque. Tri par taille, nom ou date. En mode administrateur sur NTFS, Kleaner lit directement la table des fichiers (en lecture seule) : environ 4 à 10 fois plus vite (2 s au lieu de 8 à 21 s sur un disque système de 420 Go).",
  "- **Nettoyage** : " + cat.length + " éléments intégrés et environ " + roundDown(community) + " du catalogue communautaire, chacun disant ce qu'il supprime et ce que ça change. [Liste complète](docs/catalog.fr.md).",
  "- **Vieux projets**, **doublons**, **photos et vidéos** (avec visionneuse intégrée) : libère de la place là où toi seul peux juger, avec un filet de sécurité.",
  "- **Programmes** : désinstallation par le désinstalleur officiel, taille réelle, date du dernier lancement ; retrouve les restes de programmes désinstallés et les entrées mortes de la liste.",
  "- **Démarrage** : ce qui se lance avec Windows et la durée des derniers démarrages. Désactiver un élément ne supprime rien et se défait.",
  "- **Santé et sécurité** (lecture seule) : disques, batterie, plantages, redémarrage en attente, antivirus, pare-feu, Secure Boot, BitLocker… Chaque alerte ouvre l'écran de Windows qui la règle. Pas un antivirus.",
  "- **Raccourcis cassés**, **Cloud** (OneDrive et Dropbox : garder les gros fichiers en ligne uniquement), **Mises à jour et pilotes** (Windows Update et winget, sur clic).",
  "- **Journal** de chaque action, **rapport pour une IA** (facultatif, hors ligne : un résumé Markdown à coller dans l'IA de ton choix), 6 langues, thèmes clair et sombre.",
  "",
  "## Télécharger",
  "",
  "1. Va dans [Releases](https://github.com/Swaksm/Kleaner/releases) et télécharge " + code("kleaner.exe") + " et " + code("kleaner.exe.sha256") + ".",
  "2. Vérifie l'empreinte (PowerShell). Elle doit être identique au contenu de " + code("kleaner.exe.sha256") + " :",
  "",
  hash,
  "",
  "3. Lance " + code("kleaner.exe") + ". Rien à installer : c'est un exe portable. Windows 10 ou 11 en 64 bits. La langue se change dans *Settings → Language*.",
  "",
  "Ou avec Scoop :",
  "",
  scoop,
  "",
  "**Avertissement Windows (SmartScreen).** L'exe n'est pas encore signé (un certificat coûte cher pour un projet gratuit, et les programmes de signature pour l'open source demandent d'abord plus d'étoiles). Windows affiche « Windows a protégé votre ordinateur » : clique sur *Informations complémentaires* puis *Exécuter quand même*. Certains antivirus peuvent réagir parce que Kleaner supprime des fichiers et peut demander les droits admin. Tout le code est lisible ici, et l'exe est construit par GitHub Actions à partir de ce code : l'empreinte ci-dessus prouve que tu as bien cette version.",
  "",
  "## Ce que Kleaner ne fait jamais",
  "",
  "- Agir sans ta confirmation.",
  "- Toucher à tes mots de passe, cookies, sessions ou historique.",
  "- Nettoyer ou « réparer » le registre. Il n'y écrit que sur ta demande, et seulement deux choses : l'interrupteur d'un programme au démarrage, et le retrait d'une entrée de programme morte (sauvegardée d'abord dans un fichier .reg, restaurable depuis le Journal).",
  "- Se connecter à Internet : **aucune télémétrie, aucune mise à jour automatique**.",
  "- Demander les droits admin sans que tu cliques sur « Analyse complète (admin) ».",
  "- Supprimer définitivement quoi que ce soit hors de la liste publique de caches. Les fichiers personnels vont toujours à la Corbeille.",
  "",
  "## Aide à l'améliorer",
  "",
  "- **Kleaner affiche « inconnu » pour quelque chose que tu reconnais ?** Clique sur son verdict, puis *Proposer une règle* : un formulaire GitHub s'ouvre dans ton navigateur, pré-rempli sans ton nom d'utilisateur. Tu le relis et tu décides de l'envoyer ou non. Voir [docs/rules.md](docs/rules.md) (en anglais).",
  "- **Un bug ?** *À propos → Signaler un problème* prépare un rapport sans chemins personnels.",
  "- Un cache absent du catalogue ? Ouvre un ticket avec le dossier et une source qui montre qu'il se supprime sans risque.",
  "",
  "## Retirer Kleaner",
  "",
  "*Paramètres → Retirer Kleaner* efface " + code("%LOCALAPPDATA%\\Kleaner") + " et " + code("%LOCALAPPDATA%\\com.swaksm.kleaner") + ". Supprime ensuite " + code("kleaner.exe") + ". Rien d'autre n'a été installé.",
  "",
  "## Développement",
  "",
  dev,
  "",
  "Prérequis : Node 22+, Rust stable, Visual Studio Build Tools (C++). Spécification : [SPEC.md](SPEC.md). Avant chaque release : [docs/release-checklist.md](docs/release-checklist.md). Les captures viennent d'un mode démo avec de fausses données (" + code("src/demo") + ") :",
  "",
  shots,
  "",
  "## Crédits et licences",
  "",
  "Créé par [swaks](https://github.com/Swaksm). Licence [MIT](LICENSE), sans garantie.",
  "",
  "Le catalogue communautaire (" + code("src-tauri/winapp2.json") + ") est dérivé de [Winapp2](https://github.com/MoscaDotTo/Winapp2), du projet Winapp2, sous licence [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). Voir [WINAPP2-NOTICE.md](WINAPP2-NOTICE.md). Tout le reste est sous MIT.",
  "",
].join("\n");

const forbidden = ["Cookies", "Local Storage", "IndexedDB", "Login Data", "Session Storage", "Network", "Service Worker"].map(code).join(", ");

const catalogEn = [
  "# What Kleaner can clean",
  "",
  "[Français](catalog.fr.md) · [Back to the README](../README.md)",
  "",
  "Everything Kleaner's built-in catalog can permanently delete (" + cat.length + " items). Items for software you don't have are hidden. \"Admin\": requires the full scan.",
  "",
  table("en", ["Item", "Removes", "What changes", "Admin"], "yes"),
  "",
  "Browser and app caches only target cache folders (" + ["Cache", "Code Cache", "GPUCache", "cache2"].map(code).join(", ") + "…). A test fails the build if a catalog path touches " + forbidden + ".",
  "",
  "On top of this list, a **community catalog** adds about " + roundDown(community) + " caches, temporary files and logs of other programs. It comes from [Winapp2](https://github.com/MoscaDotTo/Winapp2), filtered by Kleaner (see [WINAPP2-NOTICE.md](../WINAPP2-NOTICE.md)). Those items are never ticked for you.",
  "",
  "Want an app added? Open an issue with the cache folder and a source showing it is safe to delete.",
  "",
].join("\n");

const catalogFr = [
  "# Ce que Kleaner peut nettoyer",
  "",
  "[English](catalog.md) · [Retour au README](../README.fr.md)",
  "",
  "Tout ce que le catalogue intégré de Kleaner peut supprimer définitivement (" + cat.length + " éléments). Les éléments de logiciels que tu n'as pas sont masqués. « Admin » : demande l'analyse complète.",
  "",
  table("fr", ["Élément", "Supprime", "Ce que ça change", "Admin"], "oui"),
  "",
  "Les caches de navigateurs et d'applications ne visent que des dossiers de cache (" + ["Cache", "Code Cache", "GPUCache", "cache2"].map(code).join(", ") + "…). Un test fait échouer la compilation si un chemin du catalogue touche " + forbidden + ".",
  "",
  "En plus de cette liste, un **catalogue communautaire** ajoute environ " + roundDown(community) + " caches, fichiers temporaires et journaux d'autres programmes. Il vient de [Winapp2](https://github.com/MoscaDotTo/Winapp2), filtré par Kleaner (voir [WINAPP2-NOTICE.md](../WINAPP2-NOTICE.md)). Ces éléments ne sont jamais cochés pour toi.",
  "",
  "Tu veux qu'une appli soit ajoutée ? Ouvre un ticket avec le dossier de cache et une source qui montre qu'il se supprime sans risque.",
  "",
].join("\n");

fs.writeFileSync(`${root}/README.md`, en);
fs.writeFileSync(`${root}/README.fr.md`, fr);
fs.mkdirSync(`${root}/docs`, { recursive: true });
fs.writeFileSync(`${root}/docs/catalog.md`, catalogEn);
fs.writeFileSync(`${root}/docs/catalog.fr.md`, catalogFr);
console.log("README.md, README.fr.md, docs/catalog.md, docs/catalog.fr.md");
