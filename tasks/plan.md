# Plan : Kleaner v1

Source : [SPEC.md](../SPEC.md), révision 2.

## Approche

Le projet avance par tranches verticales : chaque phase livre quelque chose qui tourne dans l'exe.

La **sécurité vient avant les fonctions**. `guard` et `journal` existent et sont testés avant qu'une seule ligne de suppression soit écrite. Le catalogue `cleaners.json` est le seul moteur de suppression : ajouter un nettoyage revient à ajouter des données et un test, pas du code.

## Graphe des dépendances

```
P0 Socle (scaffold, style, i18n)
 └─ P1 Sécurité (guard, journal, avertissement)
     ├─ P2 Analyse (scan de tous les disques, gros fichiers, Corbeille)
     ├─ P3 Nettoyage utilisateur (moteur du catalogue, entrées sans admin, écran)
     │    └─ P4 Admin (élévation, entrées système, nettoyages heavy)
     └─ P5 Programmes (liste et désinstallation)
P6 Release (CI, README, check-list) ← tout
```

P2, P3 et P5 sont indépendants après P1. On les fait dans l'ordre P2 → P3 → P5, parce que l'écran Analyse sert de base visuelle aux autres.

## Points de contrôle

- **Après P1** : on n'a encore rien de destructeur. Tous les tests `guard` passent, y compris ceux sur les jonctions.
- **Après P2** : on mesure le temps du scan et la RAM sur un vrai PC. Si on ne tient pas la cible (60 s pour 1 million de fichiers), on corrige avant d'aller plus loin.
- **Après P3** : on teste sur une VM. On se connecte à Chrome, Discord et VS Code, on nettoie, puis on vérifie qu'on est toujours connecté. **C'est le point bloquant de la phase.**
- **Après P4** : on teste chaque nettoyage `heavy` sur un snapshot de VM.

## Risques

| Risque | Impact | Parade |
|---|---|---|
| Une suppression sort du périmètre prévu (bug de chemin, jonction) | Perte de données de l'utilisateur | Allowlist du catalogue côté Rust, refus de tout point de reparse, tests dédiés, suppression écrite à la main sans `remove_dir_all` |
| Un cache navigateur ou appli contient des données de session | L'utilisateur est déconnecté | Liste blanche stricte de noms de dossiers, test sur tout le catalogue, vérification manuelle sur VM |
| Navigateur ou appli ouvert pendant le nettoyage | Fichiers verrouillés, voire corruption | Champ `close` : si le processus tourne, l'entrée est ignorée et signalée |
| Antivirus et SmartScreen (exe non signé, suppression, admin) | Les gens n'osent pas lancer l'exe | README transparent, SHA-256 publié, code lisible, aucun réseau |
| Windows.old protégé par ses ACL | La suppression échoue | `takeown` et `icacls` uniquement sur ce dossier, tous deux affichés à l'utilisateur. À valider sur VM |
| Scan de tous les disques trop lent ou trop gourmand en RAM | Appli lourde | `jwalk` en parallèle, agrégation à la volée (on ne garde ni l'arbre complet ni la liste de tous les fichiers), top-N par tas |
| Données WebView2 d'un exe portable | Des traces restent après « Retirer Kleaner » | Dossier de données forcé dans `%LOCALAPPDATA%\Kleaner\webview` |

## Ce qui peut se faire en parallèle

- Les entrées du catalogue (T10 et T11) sont surtout de la recherche de chemins, menée en parallèle de l'écran (T12).
- Les traductions EN se font au fil de l'eau, pas à la fin.
