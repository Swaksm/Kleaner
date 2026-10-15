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
   - Lier le dépôt GitHub comme *Trusted Build System*
4. Dans GitHub → Settings → Secrets and variables → Actions :
   - Secret `SIGNPATH_API_TOKEN` : jeton d'un utilisateur CI SignPath ayant le rôle *Submitter*
   - Variable `SIGNPATH_ORGANIZATION_ID` : l'identifiant de l'organisation SignPath
5. Pousser un tag (`v0.1.0-rc6`) : le job envoie l'exe à SignPath, attend la signature, vérifie qu'elle est valide (`Get-AuthenticodeSignature`) puis publie.

Si les slugs choisis dans SignPath sont différents, les changer dans `.github/workflows/release.yml`.

## Vérifier un exe signé

```powershell
Get-AuthenticodeSignature .\kleaner.exe | Format-List Status, SignerCertificate
```
