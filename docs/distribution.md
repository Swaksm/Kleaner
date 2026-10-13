# Distribuer Kleaner : Scoop et winget

## Scoop (déjà prêt)

Le dépôt sert lui-même de « bucket » Scoop (dossier `bucket/`). N'importe qui peut installer :

```powershell
scoop bucket add kleaner https://github.com/Swaksm/Kleaner
scoop install kleaner/kleaner
```

À chaque release : mettre à jour `bucket/kleaner.json` (version, URL, hash du fichier `kleaner.exe.sha256` de la release). Le bloc `autoupdate` permet aussi à l'outil `checkver.ps1 -Update` de Scoop de le faire automatiquement, mais `checkver` ne voit que les releases **stables** (pas les `-rc`).

## winget (soumission manuelle à Microsoft)

winget lit les manifestes du dépôt public `microsoft/winget-pkgs`. Il faut y ouvrir une pull request ; Microsoft la valide automatiquement puis la fusionne.

1. Publier une release **stable** (ex. `v0.1.0`, sans `-rc`) : winget accepte les préversions, mais une première version stable passe mieux la revue.
2. Générer et valider les manifestes :

   ```powershell
   node packaging/winget/make-manifests.mjs 0.1.0
   winget validate --manifest packaging/winget/manifests/s/Swaks/Kleaner/0.1.0
   ```

3. Tester l'installation en local :

   ```powershell
   winget settings --enable LocalManifestFiles
   winget install --manifest packaging/winget/manifests/s/Swaks/Kleaner/0.1.0
   ```

4. Soumettre (au choix) :
   - avec l'outil officiel : `winget install Microsoft.WingetCreate`, puis `wingetcreate submit packaging/winget/manifests/s/Swaks/Kleaner/0.1.0` ;
   - ou à la main : forker `microsoft/winget-pkgs`, copier le dossier dans `manifests/s/Swaks/Kleaner/0.1.0/`, ouvrir la pull request.

Une fois fusionné : `winget install Swaks.Kleaner`.

Un exe signé (voir [signing.md](signing.md)) réduit les avertissements des antivirus lors de la validation automatique de Microsoft.
