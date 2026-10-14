# Kleaner

[English](README.md) · **Français**

Nettoyeur Windows gratuit, open source et **transparent**. Kleaner analyse tes disques, fichiers cachés compris, et supprime ce qu'on ne voit pas d'habitude : temporaires, caches, restes de mises à jour, dumps… **sans rien changer à ton utilisation du PC** : tu restes connecté partout, tu ne perds ni fichiers ni réglages.

## Ce que fait Kleaner

- **Analyse** les disques que tu choisis (internes, USB…) : espace libre, plus gros fichiers et dossiers, répartition par type (graphiques), tri par taille, nom ou date.
- **Dit si supprimer quelque chose casse tout** : chaque gros fichier et dossier a un verdict (supprimable, à nettoyer avec son outil, à désinstaller, casse quelque chose, tes fichiers, système, inconnu) avec ce que c'est, ce qui se passe si tu le supprimes, et comment libérer la place proprement. Règles écrites à la main dans [`knowledge.json`](src-tauri/knowledge.json) ; Kleaner ne devine jamais. Les fichiers critiques demandent une confirmation en plus.
- **Doublons** : retrouve les photos, vidéos, documents et archives identiques (comparés octet par octet) et n’en garde qu’une copie.
- **Vieux projets** : retrouve les `node_modules`, `.venv`, `target`, `build`… des projets pas touchés depuis des mois, avec la commande qui les recrée.
- **Carte du disque** : chaque dossier en rectangle à la taille de sa place ; clique pour entrer.
- **Photos & vidéos** : toutes tes photos et vidéos personnelles, triables et cherchables, en liste ou en grille, avec une visionneuse intégrée ; envoie à la Corbeille celles dont tu n'as plus besoin.
- **Nettoie** les éléments du [catalogue](#catalogue) : chacun dit ce qu'il supprime, pourquoi c'est inutile et ce que ça change. Rien n'est coché d'avance, et « Voir la liste exacte » montre chaque fichier avant toute suppression.
- **Désinstalle** des programmes via leur désinstalleur officiel, avec taille réelle et date du dernier lancement.
- **Rapport pour une IA** (optionnel, hors ligne) : un résumé Markdown + un prompt à coller dans l'IA de ton choix ; elle te pose des questions (« utilises-tu encore… ? ») et te dit quoi supprimer. Kleaner n'envoie rien.
- **Journalise** chaque action dans `%LOCALAPPDATA%\Kleaner\journal.log`.
- **Mode admin** (à la demande) : analyse complète, dossiers système protégés compris, en lecture seule.

## Ce que Kleaner ne fait jamais

- Agir sans ta confirmation.
- Toucher à tes mots de passe, cookies, sessions ou historique.
- Écrire dans le registre.
- Se connecter à Internet : **aucune télémétrie, aucune mise à jour automatique**.
- Demander les droits admin sans que tu cliques sur « Analyse complète (admin) ».
- Supprimer définitivement un fichier hors des dossiers listés dans [`src-tauri/cleaners.json`](src-tauri/cleaners.json) (vérifié en Rust par [`guard.rs`](src-tauri/src/guard.rs), couvert par des tests). Les fichiers personnels vont toujours à la Corbeille.

## Télécharger

1. Va dans [Releases](https://github.com/Swaksm/Kleaner/releases) et télécharge `kleaner.exe` et `kleaner.exe.sha256`.
2. Vérifie l'empreinte (PowerShell) :

```powershell
(Get-FileHash kleaner.exe -Algorithm SHA256).Hash.ToLower()
```

   Le résultat doit être identique au contenu de `kleaner.exe.sha256`.
3. Lance `kleaner.exe`. Rien à installer : c'est un exe portable. La langue se change dans *Settings → Language*.

Ou avec un gestionnaire de paquets :

```powershell
scoop bucket add kleaner https://github.com/Swaksm/Kleaner
scoop install kleaner/kleaner
```

(winget : bientôt, voir [docs/distribution.md](docs/distribution.md).)

**Avertissement Windows (SmartScreen).** L'exe n'est pas signé. Windows affiche « Windows a protégé votre ordinateur » : clique sur *Informations complémentaires* puis *Exécuter quand même*. Certains antivirus peuvent réagir, car Kleaner supprime des fichiers et peut demander les droits admin. Tout le code est lisible ici, et l'exe est compilé par GitHub Actions à partir de ce code.

Signature de code gratuite fournie par [SignPath.io](https://signpath.io), certificat de la [SignPath Foundation](https://signpath.org).

## Retirer Kleaner

*Paramètres → Retirer Kleaner* efface `%LOCALAPPDATA%\Kleaner` et `%LOCALAPPDATA%\com.swaksm.kleaner`. Supprime ensuite `kleaner.exe`.

## Catalogue

Tout ce que Kleaner peut supprimer définitivement. Les éléments des logiciels que tu n'as pas sont simplement masqués. « Admin » : nécessite l'analyse complète.

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
| Vivaldi : cache | Copies locales des pages, scripts et shaders déjà chargés. | Tu restes connecté. Sites un peu plus lents au premier chargement. |  |
| Chromium : cache | Copies locales des pages, scripts et shaders déjà chargés. | Tu restes connecté. Sites un peu plus lents au premier chargement. |  |
| Steam : cache web et shaders | Pages de la boutique déjà vues et shaders compilés pour tes jeux. | Les jeux peuvent saccader un peu au premier lancement. |  |
| Epic Games Launcher : cache web | Pages et images de la boutique déjà chargées par le launcher. | Tu restes connecté. La boutique charge un peu plus lentement une fois. |  |
| Adobe : cache média | Fichiers d’aperçu audio et vidéo créés par Premiere Pro, After Effects et Media Encoder. | Les projets mettent plus de temps à s’ouvrir la première fois. |  |
| Maven : cache | Paquets et résultats de build déjà téléchargés par Maven. | Le prochain install ou build retélécharge ce qui manque. |  |
| Go : cache | Paquets et résultats de build déjà téléchargés par Go. | Le prochain install ou build retélécharge ce qui manque. |  |
| Bun : cache | Paquets et résultats de build déjà téléchargés par Bun. | Le prochain install ou build retélécharge ce qui manque. |  |
| Composer : cache | Paquets et résultats de build déjà téléchargés par Composer. | Le prochain install ou build retélécharge ce qui manque. |  |
| Dart/Flutter (pub) : cache | Paquets et résultats de build déjà téléchargés par Dart/Flutter (pub). | Le prochain install ou build retélécharge ce qui manque. |  |
| Installeurs de pilotes graphiques | Paquets de pilotes extraits par les installeurs NVIDIA et AMD (C:\NVIDIA, C:\AMD, téléchargements NVIDIA). | Rien. Une réinstallation du pilote les retélécharge. | oui |

## Crédits

Créé par [swaks](https://github.com/Swaksm). Licence [MIT](LICENSE), sans garantie.
