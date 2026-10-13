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

## Programmes

- [ ] Liste triée par taille, recherche fonctionnelle.
- [ ] Désinstaller un petit programme (ex. 7-Zip) : le désinstalleur officiel s'ouvre, le programme disparaît après rafraîchissement.

## Fin

- [ ] Journal : toutes les actions ci-dessus y sont, avec tailles et erreurs.
- [ ] Paramètres → Retirer Kleaner : l'appli se ferme, `%LOCALAPPDATA%\Kleaner` et `%LOCALAPPDATA%\com.swaksm.kleaner` n'existent plus.
- [ ] Aucune connexion réseau pendant toute la session (Moniteur de ressources → Réseau).
- [ ] Publier la release finale (tag `v*` sans `-rc`).
