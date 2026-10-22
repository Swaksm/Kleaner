# Tests manuels avant v0.1.0

Sur la vraie machine, avec la release v0.1.0-rc14. Cocher `[x]` quand c'est bon ; noter ce qui casse sous la ligne.
Avant de commencer : créer un point de restauration Windows (les nettoyages lourds sont irréversibles).

## 1. Installation

- [ ] **Installateur** : lancer `kleaner-setup.exe`. → Invite UAC, choix de la langue de l'installateur, installation dans `C:\Program Files\Kleaner`, raccourci dans le menu Démarrer.
- [ ] **Fin de l'installation** : cocher « Lancer Kleaner ». → Kleaner s'ouvre (sans être admin).
- [ ] **Paramètres Windows → Applications** : Kleaner apparaît avec son icône, l'éditeur « swaks » et la version.
- [ ] **Exe portable** : lancer `kleaner.exe` téléchargé à côté. → S'ouvre aussi ; une seule fenêtre Kleaner à la fois (le 2e lancement ramène la 1re).
- [ ] **SmartScreen** : au 1er lancement, Windows affiche « Éditeur inconnu ». → « Informations complémentaires → Exécuter quand même » fonctionne (attendu tant que l'exe n'est pas signé).

## 2. Premier lancement

- [ ] **Avertissement** : logo, 6 points, bouton grisé tant que « J'ai lu et compris » n'est pas coché.
- [ ] **Assistant** : logo, 6 drapeaux. Cliquer « Français » → tout l'écran passe en français immédiatement.
- [ ] **Thème** dans l'assistant : « Clair » puis « Sombre » → la fenêtre change tout de suite.
- [ ] **Mises à jour** : la case est décochée par défaut.
- [ ] **Démarrage avec Windows** : la case est décochée par défaut.
- [ ] **Terminer** sans cocher le démarrage → arrivée sur l'Accueil, aucune invite UAC.
- [ ] **Relancer Kleaner** → ni l'avertissement ni l'assistant ne reviennent.

## 3. Interface générale

- [ ] **Logo** à côté de « Kleaner » dans la barre latérale, de la couleur d'accent (lisible en clair et en sombre).
- [ ] **Icônes** devant chaque page, alignées avec le texte.
- [ ] **Nettoyage** est un bouton plein dans la barre latérale.
- [ ] **Drapeau** à côté de l'engrenage : clic → liste des 6 langues vers le haut ; choisir une langue → change tout ; Échap ou clic à côté → se ferme.
- [ ] **Effet Mica** (Windows 11) : le fond d'écran se devine, flouté, sous la barre latérale ; le reste est opaque. Fenêtre inactive → fond uni (normal).
- [ ] **Windows 10** (si tu as une machine) : aucune zone transparente, tout est opaque.
- [ ] **Raccourcis** : Ctrl+1 à Ctrl+8 ouvrent les 8 pages principales (le survol d'une page affiche « Ctrl+N »).
- [ ] **F5** : lance une analyse (la fenêtre ne se recharge pas). F5 pendant une analyse → pas de deuxième analyse.
- [ ] **Redimensionner** la fenêtre au minimum (800×560) → rien ne déborde ni ne se chevauche.

## 4. Accueil et Analyse

- [ ] **Accueil sans analyse** : disques avec barre colorée (vert < 75 %, orange 75–90 %, rouge > 90 %) et bouton « Analyser ».
- [ ] **Analyse** : progression qui avance (fichiers, Go), puis résultats : plus gros fichiers et dossiers avec verdict, répartition, carte du disque.
- [ ] **Annuler** une analyse en cours → s'arrête proprement.
- [ ] **Analyse admin** (« Analyse complète (admin) ») : invite UAC, badge Admin, analyse NTFS rapide (quelques secondes sur C:).
- [ ] **Verdict** : clic sur un verdict → fiche (ce que c'est, ce qui se passe si on supprime). « Inconnu » → bouton « Proposer une règle » ouvre GitHub.
- [ ] **Ouvrir** sur un gros fichier → l'Explorateur s'ouvre sur le fichier. Aucune case ni Corbeille dans ces listes.
- [ ] **Accueil après analyse** : « Nettoyer sans risque (X Go) » + les recommandations avec leurs tailles.

## 5. Nettoyage

- [ ] **Chargement** : blocs gris animés, puis le graphique « Espace récupérable par type ».
- [ ] **Rien n'est coché** à l'ouverture de la page.
- [ ] **« Cocher les éléments sans risque »** → cases cochées, total mis à jour.
- [ ] **« Voir la liste exacte »** → liste des fichiers ; le fichier complet s'ouvre (dans `%LOCALAPPDATA%\Kleaner\rapports`).
- [ ] **Navigateur ouvert** (Chrome/Edge/Firefox) → son cache est grisé « Ferme d'abord… ».
- [ ] **Nettoyer** → confirmation avec la liste, puis carte verte en haut : coche, « X Go libérés », espace libre avant → après pour chaque disque concerné.
- [ ] **Après le nettoyage** : tu restes connecté à tes sites, rien de personnel n'a disparu.
- [ ] **Accueil → « Nettoyer sans risque »** → ouvre Nettoyage avec les éléments sûrs cochés et la confirmation ouverte. Annuler → rien n'est supprimé.
- [ ] **Journal** : une ligne par nettoyage, avec fichiers et taille.

### Nettoyages lourds (admin, jamais testés : un à la fois)

- [ ] **Corbeille** : vidée, taille annoncée = taille libérée.
- [ ] **MEMORY.DMP** (s'il existe) : supprimé.
- [ ] **Cache Windows Update** : supprimé, puis Windows Update fonctionne encore (Paramètres → Windows Update → Rechercher).
- [ ] **Optimisation de la distribution** : vidée.
- [ ] **Windows.old** (s'il existe) : dossier supprimé, avertissement affiché avant.
- [ ] **Magasin de composants (DISM)** : se termine (long), sortie visible.
- [ ] **Hibernation** : `hiberfil.sys` disparaît ; « Démarrage rapide » est coupé (avertissement affiché avant).
- [ ] **Points de restauration** : tous supprimés sauf le plus récent.

## 6. Photos & vidéos, Doublons

- [ ] **Photos & vidéos** : liste après analyse, tri, aperçu d'une photo et d'une vidéo, « Ouvrir » lance l'appli par défaut.
- [ ] **Corbeille** depuis Photos & vidéos → fichier dans la Corbeille, ligne au Journal.
- [ ] **Doublons** : recherche avec progression, groupes du plus gaspillé au moins gaspillé.
- [ ] **Impossible de cocher toutes les copies** d'un groupe (la dernière reste décochable).
- [ ] **« Tout cocher sauf la plus ancienne »** → une copie reste par groupe.
- [ ] **Jamais de .dll, .exe ni base de données** dans les doublons.

## 7. Programmes

- [ ] **Liste** : taille réelle (blocs gris pendant la mesure), dernier lancement (admin).
- [ ] **Désinstaller** un petit programme → son désinstalleur officiel s'ouvre ; Journal mis à jour.
- [ ] **Restes** : liste des restes, envoi à la Corbeille, restaurable. Android : avertissement Flutter.
- [ ] **Entrées fantômes** (jamais testé) : supprimer une entrée → sauvegarde `.reg` créée, entrée disparue de Paramètres → Applications. **Restaurer** depuis le Journal → elle revient.

## 8. Démarrage

- [ ] **Liste** : programmes au démarrage (registre, dossiers, tâches) ; services en lecture seule.
- [ ] **Désactiver** un élément (jamais testé) → Gestionnaire des tâches → Démarrage le montre « Désactivé ». **Réactiver** → « Activé ».
- [ ] **Durées de démarrage** (admin, jamais testé) : s'affichent.
- [ ] **Programme absent** : un élément dont l'exe n'existe plus est signalé.

## 9. Santé, Mises à jour, autres outils

- [ ] **Santé & sécurité** : bilan en lecture seule ; chaque bouton ouvre le bon écran Windows. « Vérifier l'image (DISM) » se termine.
- [ ] **Windows Update** (jamais testé) : la recherche affiche les mises à jour en attente ; le bouton ouvre Windows Update.
- [ ] **Mises à jour winget** (jamais testé) : liste des applis ; en mettre une à jour → réussit ; une appli au nom très long → noter si l'identifiant finit par « … ».
- [ ] **Raccourcis cassés** : liste, Corbeille, restauration. En admin, un raccourci du menu Démarrer commun (`C:\ProgramData\…`) part bien à la Corbeille (issue #1).
- [ ] **Vieux projets** (si analyse en a trouvé) : suppression des node_modules / target choisis, le projet reste.
- [ ] **OneDrive** (jamais testé dans l'exe) : « Libérer de l'espace » → icône nuage dans l'Explorateur, espace libéré quelques minutes après.
- [ ] **Journal** : restaurer un fichier envoyé à la Corbeille → il revient avec son extension.

## 10. Démarrage avec Windows (jamais testé)

- [ ] **Paramètres → Démarrage** : cocher sans être admin → invite UAC, Kleaner se relance en admin, la case est cochée.
- [ ] **Refuser l'UAC** à cette étape → la case reste décochée. Plus tard, relancer en admin pour autre chose → la tâche ne doit **pas** s'activer toute seule.
- [ ] **Planificateur de tâches** : une tâche « Kleaner » existe (à l'ouverture de session, privilèges élevés).
- [ ] **Redémarrer le PC** → pas d'invite UAC, pas de fenêtre ; une icône Kleaner près de l'horloge.
- [ ] **Clic gauche** sur l'icône → la fenêtre s'ouvre (badge Admin). **Fermer** la fenêtre → elle retourne près de l'horloge.
- [ ] **Clic droit → Quitter** → l'icône disparaît, Kleaner est fermé.
- [ ] **Lancer Kleaner depuis le menu Démarrer** pendant que l'icône tourne → la fenêtre existante doit s'afficher (à vérifier : Windows peut bloquer entre admin et non-admin).
- [ ] **Notification** (disques fixes seulement) : si un disque a moins de 10 % libre au démarrage → notification « Disque presque plein » dans les 30 min ; noter le nom affiché (Kleaner ou Windows PowerShell).
- [ ] **Décocher** dans Paramètres → la tâche disparaît du Planificateur. Journal : `autostart-on` / `autostart-off`.

## 11. Paramètres, À propos, désinstallation

- [ ] **Version** (à partir de la rc15) : À propos affiche « 0.1.0-rc15 », pas « 0.1.0 ». La rc14 affiche encore « 0.1.0 » (corrigé après).
- [ ] **Vérification des mises à jour** (rc15 installée, une rc16 publiée) : À propos → « Vérifier » propose la rc16.
- [ ] **Messages d'erreur** en français : par ex. Doublons → tout envoyer à la Corbeille après avoir supprimé une copie à la main → « une copie de chaque fichier reste toujours ».
- [ ] **Rapport de bug** : crée un fichier sans chemins personnels, ouvre le formulaire GitHub.
- [ ] **Rapport pour IA** : fichier Markdown créé.
- [ ] **Retirer Kleaner** avec le démarrage auto actif et sans admin → refus clair, rien n'est effacé.
- [ ] **Retirer Kleaner** (démarrage auto coupé) → Kleaner se ferme ; `%LOCALAPPDATA%\Kleaner` et `%LOCALAPPDATA%\com.swaksm.kleaner` ont disparu ; Kleaner installé dans Program Files est toujours là.
- [ ] **Désinstaller** depuis Paramètres Windows → Applications → `C:\Program Files\Kleaner` disparaît, raccourci du menu Démarrer aussi.

## 12. Distribution

- [ ] **Scoop** : `scoop install kleaner` (bucket du dépôt) → installe la rc14, lance l'exe.
- [ ] **Empreintes** : `Get-FileHash` des deux exe = contenu des fichiers `.sha256` de la release.
