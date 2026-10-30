# Check-list de release

À faire sur une **VM Windows 10** et une **VM Windows 11**, à partir de l'exe téléchargé depuis la Release candidate (`v*-rc*`), pas d'un build local.

## Préparation

- [ ] Snapshot de la VM avant de commencer.
- [ ] Installer et se connecter : Chrome, Edge, Firefox, Discord, VS Code (compte GitHub).
- [ ] Naviguer un peu dans chaque appli pour remplir les caches.

## Téléchargement

- [ ] Le SHA-256 de `kleaner.exe` correspond à `kleaner.exe.sha256`.
- [ ] SmartScreen s'affiche, « Exécuter quand même » fonctionne.
- [ ] Taille de l'exe < 15 Mo, démarrage < 2 s.

## Premier lancement

- [ ] L'avertissement s'affiche, « Continuer » est bloqué tant que la case n'est pas cochée.
- [ ] FR et EN complets, thème clair et sombre, navigation au clavier (focus visible).

## Analyse

- [ ] L'analyse de C: se termine, progression visible, Annuler fonctionne.
- [ ] Mesurer : temps, nombre de fichiers, RAM max (Gestionnaire des tâches) < 250 Mo.
- [ ] « Ouvrir » ouvre l'Explorateur sur le bon fichier.
- [ ] Envoyer un fichier factice à la Corbeille : il y est, l'action est dans le Journal.
- [ ] Essayer un fichier de `C:\Windows` (via un fichier mis en avant) : refusé.

## Nettoyage (point bloquant)

- [ ] Fermer toutes les applis, cocher toutes les entrées `safe`, nettoyer.
- [ ] Rouvrir Chrome, Edge, Firefox, Discord, VS Code : **toujours connecté partout**.
- [ ] Une appli ouverte (Chrome) bloque son entrée avec « Ferme d'abord : chrome.exe ».
- [ ] Le rapport indique l'espace libéré et les fichiers ignorés.

## Admin

- [ ] « Analyse complète (admin) » : UAC accepté → Kleaner se relance en mode admin.
- [ ] UAC refusé → Kleaner reste ouvert en mode utilisateur, message d'erreur.
- [ ] Entrées système : nettoyées, Windows Update fonctionne toujours ensuite.

## Nettoyages lourds (sur snapshot, un par un)

- [ ] Windows.old (VM mise à jour depuis une version précédente) : supprimé, Windows démarre.
- [ ] WinSxS (DISM) : se termine, sortie dans le Journal.
- [ ] Hibernation : `hiberfil.sys` disparu, `powercfg /hibernate on` le rétablit.
- [ ] Points de restauration : il en reste exactement un (le plus récent).
- [ ] Corbeille : vidée après la double confirmation.
- [ ] Case « Créer d'abord un point de restauration » cochée : un point « Kleaner » apparaît dans `rstrui` (ou le message « point de moins de 24 h déjà présent ») ; ligne `restore-point` dans le Journal.
- [ ] Protection du système coupée sur C: : la question « Continuer sans ? » s'affiche ; Annuler ne lance rien.
- [ ] CompactOS : se termine (10 à 30 min), l'espace gagné est affiché, Windows démarre ; `compact /CompactOS:never` l'annule.

## Programmes

- [ ] Liste triée par taille, recherche fonctionnelle.
- [ ] Désinstaller un petit programme (ex. 7-Zip) : le désinstalleur officiel s'ouvre, le programme disparaît après rafraîchissement.
- [ ] Applis préinstallées : la liste ne contient ni Store, ni Edge, ni Photos, ni Calculatrice ; rien n'est coché.
- [ ] Retirer Solitaire : il disparaît du menu Démarrer, ligne `remove-app` dans le Journal ; « Ouvrir dans le Store » ouvre sa page et la réinstallation marche.

## Conseils

- [ ] Placer un vieil installeur dans Téléchargements (date > 30 jours) et un `photos.zip` à côté d'un dossier `photos` : tous deux listés.
- [ ] Les envoyer à la Corbeille, puis les restaurer depuis le Journal.
- [ ] Steam installé avec un jeu pas lancé depuis 6 mois : « Désinstaller avec Steam » ouvre la confirmation de Steam.

## Paramètres et rappel

- [ ] La case du rappel mensuel se coche et se décoche, et reste après une relance.
- [ ] Démarrage avec Windows activé, fichier `%LOCALAPPDATA%\Kleaner\reminder` supprimé : 30 min après l'ouverture de session, la notification « Place à libérer » s'affiche (si plus de 1 Go), pas avant.

## Fin

- [ ] Journal : toutes les actions ci-dessus y sont, avec tailles et erreurs.
- [ ] Paramètres → Retirer Kleaner : l'appli se ferme, `%LOCALAPPDATA%\Kleaner` et `%LOCALAPPDATA%\com.swaksm.kleaner` n'existent plus.
- [ ] Aucune connexion réseau pendant toute la session (Moniteur de ressources → Réseau).
- [ ] Publier la release finale (tag `v*` sans `-rc`).
