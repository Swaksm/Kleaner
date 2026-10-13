// Génère les manifestes winget d'une version publiée : node packaging/winget/make-manifests.mjs 0.1.0
// Le hash est lu dans kleaner.exe.sha256 de la release GitHub. Sortie : packaging/winget/manifests/…
import { mkdirSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!version) throw new Error("usage: node make-manifests.mjs <version>");
const base = `https://github.com/Swaksm/Kleaner/releases/download/v${version}`;
const res = await fetch(`${base}/kleaner.exe.sha256`);
if (!res.ok) throw new Error(`release v${version} introuvable (${res.status})`);
const sha = (await res.text()).trim().split(/\s+/)[0].toUpperCase();
if (!/^[0-9A-F]{64}$/.test(sha)) throw new Error(`hash invalide : ${sha}`);

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
  "InstallerType: portable",
  "Commands:",
  "  - kleaner",
  "Installers:",
  "  - Architecture: x64",
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
console.log(`manifestes écrits dans ${dir} (sha256 ${sha})`);
