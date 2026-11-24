# Signer kleaner.exe (supprimer l'avertissement SmartScreen)

> **État (octobre 2026) :** SignPath a répondu qu'il faut plus d'étoiles et de visibilité avant d'accepter le projet. L'exe reste non signé ; à redemander plus tard. Le README ne mentionne donc pas de signature.

Un exe signé affiche « Éditeur : … » au lieu de « Éditeur inconnu ». La réputation SmartScreen se construit ensuite avec les téléchargements.

**SignPath Foundation** signe gratuitement les projets open source. Le workflow de release est déjà prêt : il signe automatiquement dès que les secrets existent, et publie sans signature sinon.

## Étapes (à faire par le mainteneur)

1. Lire les conditions : https://signpath.org/terms (projet open source, licence OSI comme MIT, builds faits par CI publique, release documentée). Kleaner les remplit.
2. Postuler : https://signpath.org/apply avec le dépôt `https://github.com/Swaksm/Kleaner`.
3. Une fois accepté, dans SignPath :
   - Projet avec le slug `kleaner`
   - Politique de signature `release-signing`
   - Configuration d'artefact `portable-exe` qui signe le fichier `kleaner.exe` à la racine de l'artefact (Authenticode)
   - Configuration d'artefact `installer` qui signe le fichier `kleaner-setup.exe` à la racine de l'artefact (Authenticode)
   - Dans les deux : nom de produit `Kleaner` et version imposés (restrictions de métadonnées, exigées par SignPath Foundation)
   - Lier le dépôt GitHub comme *Trusted Build System*
4. Dans GitHub → Settings → Secrets and variables → Actions :
   - Secret `SIGNPATH_API_TOKEN` : jeton d'un utilisateur CI SignPath ayant le rôle *Submitter*
   - Variable `SIGNPATH_ORGANIZATION_ID` : l'identifiant de l'organisation SignPath
5. Pousser un tag (`v1.4.0`) : le job signe en deux temps, puis publie.
   1. `tauri build --no-bundle` produit l'exe seul ; il part chez SignPath et revient signé.
   2. `tauri bundle --bundles nsis` construit l'installateur autour de cet exe signé (sans recompiler) ; l'installateur part à son tour chez SignPath.
   3. Les deux signatures sont vérifiées (`Get-AuthenticodeSignature`) avant la publication.

   Chaque signature demande une approbation manuelle dans SignPath (règle de SignPath Foundation) : deux approbations par release.

Si les slugs choisis dans SignPath sont différents, les changer dans `.github/workflows/release.yml`.

## Vérifier un exe signé

```powershell
Get-AuthenticodeSignature .\kleaner.exe | Format-List Status, SignerCertificate
```
