# Ce que Kleaner peut nettoyer

[English](catalog.md) · [Retour au README](../README.fr.md)

Tout ce que le catalogue intégré de Kleaner peut supprimer définitivement (46 éléments). Les éléments de logiciels que tu n'as pas sont masqués. « Admin » : demande l'analyse complète.

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
| Compresser Windows (CompactOS) | Les fichiers système de Windows, compressés sur place par l’outil officiel de Windows (compact /CompactOS:always). | Windows peut être un peu plus lent sur un vieux PC. Annulable avec compact /CompactOS:never. | oui |
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
| uv : cache | Paquets Python téléchargés et compilés par uv. | Le prochain install retélécharge ce qui manque. |  |
| Poetry : cache | Paquets Python déjà téléchargés par Poetry. | Le prochain install retélécharge ce qui manque. |  |
| Cypress : binaires | Le navigateur de test Cypress téléchargé pour tes projets. | Cypress se retélécharge au prochain lancement des tests. |  |
| Installeurs de pilotes graphiques | Paquets de pilotes extraits par les installeurs NVIDIA et AMD (C:\NVIDIA, C:\AMD, téléchargements NVIDIA). | Rien. Une réinstallation du pilote les retélécharge. | oui |

Les caches de navigateurs et d'applications ne visent que des dossiers de cache (`Cache`, `Code Cache`, `GPUCache`, `cache2`…). Un test fait échouer la compilation si un chemin du catalogue touche `Cookies`, `Local Storage`, `IndexedDB`, `Login Data`, `Session Storage`, `Network`, `Service Worker`.

En plus de cette liste, un **catalogue communautaire** ajoute environ 1100 caches, fichiers temporaires et journaux d'autres programmes. Il vient de [Winapp2](https://github.com/MoscaDotTo/Winapp2), filtré par Kleaner (voir [WINAPP2-NOTICE.md](../WINAPP2-NOTICE.md)). Ces éléments ne sont jamais cochés pour toi.

Tu veux qu'une appli soit ajoutée ? Ouvre un ticket avec le dossier de cache et une source qui montre qu'il se supprime sans risque.
