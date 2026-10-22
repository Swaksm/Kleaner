// Génère les manifestes winget d'une version publiée : node packaging/winget/make-manifests.mjs 0.1.0
// Les hash sont lus dans kleaner-setup.exe.sha256 et kleaner.exe.sha256 de la release GitHub.
// Deux installeurs : l'installateur NSIS (choisi par défaut par winget) et l'exe portable (--installer-type portable).
// Sortie : packaging/winget/manifests/…
import { mkdirSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!version) throw new Error("usage: node make-manifests.mjs <version>");
const base = `https://github.com/Swaksm/Kleaner/releases/download/v${version}`;
async function sha256(file) {
  const res = await fetch(`${base}/${file}.sha256`);
  if (!res.ok) throw new Error(`${file} introuvable dans la release v${version} (${res.status})`);
  const sha = (await res.text()).trim().split(/\s+/)[0].toUpperCase();
  if (!/^[0-9A-F]{64}$/.test(sha)) throw new Error(`hash invalide pour ${file} : ${sha}`);
  return sha;
}
const setupSha = await sha256("kleaner-setup.exe");
const sha = await sha256("kleaner.exe");

const id = "Swaks.Kleaner";
const schema = (k) => `# yaml-language-server: $schema=https://aka.ms/winget-manifest.${k}.1.6.0.schema.json`;
const dir = `packaging/winget/manifests/s/Swaks/Kleaner/${version}`;
mkdirSync(dir, { recursive: true });

writeFileSync(`${dir}/${id}.yaml`, [
  schema("version"),
  `PackageIdentifier: ${id}`,
  `PackageVersion: ${version}`,
  "DefaultLocale: en-US",
  "ManifestType: version",
  "ManifestVersion: 1.6.0",
  "",
].join("\n"));

writeFileSync(`${dir}/${id}.installer.yaml`, [
  schema("installer"),
  `PackageIdentifier: ${id}`,
  `PackageVersion: ${version}`,
  "Installers:",
  "  - Architecture: x64",
  "    InstallerType: nullsoft",
  "    Scope: machine",
  "    ElevationRequirement: elevatesSelf",
  `    InstallerUrl: ${base}/kleaner-setup.exe`,
  `    InstallerSha256: ${setupSha}`,
  "  - Architecture: x64",
  "    InstallerType: portable",
  "    Commands:",
  "      - kleaner",
  `    InstallerUrl: ${base}/kleaner.exe`,
  `    InstallerSha256: ${sha}`,
  "ManifestType: installer",
  "ManifestVersion: 1.6.0",
  "",
].join("\n"));

writeFileSync(`${dir}/${id}.locale.en-US.yaml`, [
  schema("defaultLocale"),
  `PackageIdentifier: ${id}`,
  `PackageVersion: ${version}`,
  "PackageLocale: en-US",
  "Publisher: swaks",
  "PublisherUrl: https://github.com/Swaksm",
  "PackageName: Kleaner",
  "PackageUrl: https://github.com/Swaksm/Kleaner",
  "License: MIT",
  "LicenseUrl: https://github.com/Swaksm/Kleaner/blob/main/LICENSE",
  "ShortDescription: Free, open-source and transparent Windows disk cleaner.",
  "Description: Scans your drives, explains what takes up space and whether deleting it breaks anything, and safely removes caches and temporary files. No telemetry, no network.",
  "Tags:",
  "  - cleaner",
  "  - disk-space",
  "  - disk-cleanup",
  "  - storage",
  "  - open-source",
  "ReleaseNotesUrl: https://github.com/Swaksm/Kleaner/releases/tag/v" + version,
  "ManifestType: defaultLocale",
  "ManifestVersion: 1.6.0",
  "",
].join("\n"));
console.log(`manifestes écrits dans ${dir} (setup ${setupSha}, portable ${sha})`);
