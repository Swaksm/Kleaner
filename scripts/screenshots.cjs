// Captures d'écran du README : `npm run demo:build && npm run demo:shots`.
// L'interface tourne en mode « demo » (données fictives, voir src/demo) dans Chrome sans fenêtre.
// CHROME=<chemin de chrome.exe> si Chrome n'est pas à l'emplacement habituel.
// --store : captures du Microsoft Store, 1920 × 1080 en anglais et en français, dans docs/store/screenshots/<langue>.
const { spawn, execFileSync } = require("node:child_process");
const { existsSync, mkdirSync } = require("node:fs");
const path = require("node:path");

const root = path.join(__dirname, "..");
const store = process.argv.includes("--store");
// Store : même mise en page (1280 × 720 points) agrandie 1,5 fois, au-dessus du minimum de 1366 × 768 pixels.
const runs = store
  ? ["en", "fr"].map((lang) => ({ lang, out: path.join(root, "docs", "store", "screenshots", lang), size: "1280,720", scale: "1.5" }))
  : [{ lang: "en", out: path.join(root, "docs", "img"), size: "1280,820", scale: "1" }];
const chrome =
  process.env.CHROME ||
  ["C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe", "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe"].find(existsSync);
if (!chrome) throw new Error("Chrome ou Edge introuvable : définis CHROME=<chemin>");

const shots = [
  { name: "home", page: "home" },
  { name: "verdicts", page: "analyse", extra: "&autoscan=1&scroll=.search" },
  { name: "cleanup", page: "cleanup" },
  { name: "startup", page: "startup" },
  { name: "health", page: "health" },
];
const port = 4173;

async function waitForServer() {
  for (let i = 0; i < 60; i++) {
    try {
      if ((await fetch(`http://127.0.0.1:${port}/`)).ok) return;
    } catch {}
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error("le serveur de démonstration ne répond pas");
}

(async () => {
  const server = spawn("npx", ["vite", "preview", "--outDir", "dist-demo", "--port", String(port), "--strictPort", "--host", "127.0.0.1"], {
    cwd: root,
    shell: true,
    stdio: "ignore",
  });
  try {
    await waitForServer();
    for (const run of runs) {
      mkdirSync(run.out, { recursive: true });
      for (const s of shots) {
        const file = path.join(run.out, `${s.name}.png`);
        execFileSync(chrome, [
          "--headless=new",
          "--disable-gpu",
          "--hide-scrollbars",
          `--force-device-scale-factor=${run.scale}`,
          `--window-size=${run.size}`,
          "--virtual-time-budget=8000",
          `--screenshot=${file}`,
          `http://127.0.0.1:${port}/?page=${s.page}&theme=dark&lang=${run.lang}${s.extra || ""}`,
        ]);
        console.log("ok", path.relative(root, file));
      }
    }
  } finally {
    try {
      execFileSync("taskkill", ["/PID", String(server.pid), "/T", "/F"], { stdio: "ignore" });
    } catch {}
  }
})().catch((e) => {
  console.error(e.message);
  process.exit(1);
});
