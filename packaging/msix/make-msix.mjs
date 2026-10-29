// Paquet Microsoft Store : npm run tauri build -- --no-bundle, puis node packaging/msix/make-msix.mjs
// Même exe que la version portable (il se reconnaît dans son paquet, voir store.rs). Pas de signature : le Store signe.
// Il faut makeappx.exe (SDK Windows 10/11). Sortie : src-tauri/target/msix/Kleaner_<version>.0_x64.msix
import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";

const root = path.join(import.meta.dirname, "..", "..");
const exe = path.join(root, "src-tauri", "target", "release", "kleaner.exe");
if (!existsSync(exe)) throw new Error(`${exe} introuvable : lance d'abord npm run tauri build -- --no-bundle`);

// Le Store veut quatre nombres, le dernier à 0, et refuse les préversions.
const { version } = JSON.parse(readFileSync(path.join(root, "src-tauri", "tauri.conf.json"), "utf8"));
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error(`version ${version} : seulement une version stable pour le Store`);
const full = `${version}.0`;

const kits = "C:\\Program Files (x86)\\Windows Kits\\10\\bin";
const sdk = existsSync(kits)
  ? readdirSync(kits)
      .filter((d) => /^10\./.test(d) && existsSync(path.join(kits, d, "x64", "makeappx.exe")))
      .sort((a, b) => a.localeCompare(b, undefined, { numeric: true }))
      .pop()
  : undefined;
if (!sdk) throw new Error("makeappx.exe introuvable : installe le SDK Windows");
const makeappx = path.join(kits, sdk, "x64", "makeappx.exe");

const out = path.join(root, "src-tauri", "target", "msix");
const layout = path.join(out, "layout");
rmSync(layout, { recursive: true, force: true });
mkdirSync(path.join(layout, "Assets"), { recursive: true });
copyFileSync(exe, path.join(layout, "kleaner.exe"));
for (const icon of ["StoreLogo.png", "Square44x44Logo.png", "Square71x71Logo.png", "Square150x150Logo.png"]) {
  copyFileSync(path.join(root, "src-tauri", "icons", icon), path.join(layout, "Assets", icon));
}
const manifest = readFileSync(path.join(import.meta.dirname, "AppxManifest.xml"), "utf8").replace('Version="{version}"', `Version="${full}"`);
writeFileSync(path.join(layout, "AppxManifest.xml"), manifest);

const msix = path.join(out, `Kleaner_${full}_x64.msix`);
// makeappx vérifie le manifeste avec le schéma officiel.
execFileSync(makeappx, ["pack", "/o", "/d", layout, "/p", msix], { stdio: "inherit" });
console.log(msix);
