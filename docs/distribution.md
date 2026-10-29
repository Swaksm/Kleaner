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

1. Publier une release **stable** (ex. `v1.0.0`, sans `-rc`) : winget accepte les préversions, mais une première version stable passe mieux la revue.
2. Générer et valider les manifestes :

   ```powershell
   node packaging/winget/make-manifests.mjs 1.1.0
   ```

   Le manifeste propose l'installateur NSIS (`kleaner-setup.exe`, choisi par défaut) et l'exe portable (`winget install Swaks.Kleaner --installer-type portable`). La release doit donc contenir les deux (à partir de v0.1.0-rc14).

   ```powershell
   winget validate --manifest packaging/winget/manifests/s/Swaks/Kleaner/1.1.0
   ```

3. Tester l'installation en local :

   ```powershell
   winget settings --enable LocalManifestFiles
   winget install --manifest packaging/winget/manifests/s/Swaks/Kleaner/1.1.0
   ```

4. Soumettre (au choix) :
   - avec l'outil officiel : `winget install Microsoft.WingetCreate`, puis `wingetcreate submit packaging/winget/manifests/s/Swaks/Kleaner/1.1.0` ;
   - ou à la main : forker `microsoft/winget-pkgs`, copier le dossier dans `manifests/s/Swaks/Kleaner/1.1.0/`, ouvrir la pull request.

Une fois fusionné : `winget install Swaks.Kleaner`.

Un exe signé (voir [signing.md](signing.md)) réduit les avertissements des antivirus lors de la validation automatique de Microsoft.

## Microsoft Store (MSIX)

Identité Partner Center : `Swaks.Kleaner`, éditeur `CN=6258F58B-4AB8-4F17-B676-8F34E41CF25D`, Store ID `9N00P7TVWDMM`, PFN `Swaks.Kleaner_dq2s3tjsjfehg`.

1. `npm run tauri build -- --no-bundle` (version stable, sans `VITE_APP_VERSION`), puis `npm run msix` → `src-tauri/target/msix/Kleaner_<version>.0_x64.msix`, non signé (le Store signe).
2. Partner Center → nouvelle soumission → Packages : déposer le `.msix`, coller la justification de `docs/store/capabilities.md`.
3. Fiche : `docs/store/listing.md`, captures `npm run demo:build && npm run store:shots` → `docs/store/screenshots/`. Politique de confidentialité : `docs/store/privacy.html`, publiée sur `https://swaks.org/kleaner/privacy/`.

Même exe que la version portable : dans son paquet (`store.rs`), il cache la mise à jour intégrée et passe par la `StartupTask` du paquet au lieu de la tâche planifiée.
