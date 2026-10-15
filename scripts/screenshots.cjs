// Captures d'écran du README : `npm run demo:build && npm run demo:shots`.
// L'interface tourne en mode « demo » (données fictives, voir src/demo) dans Chrome sans fenêtre.
// CHROME=<chemin de chrome.exe> si Chrome n'est pas à l'emplacement habituel.
const { spawn, execFileSync } = require("node:child_process");
const { existsSync, mkdirSync } = require("node:fs");
const path = require("node:path");

const root = path.join(__dirname, "..");
const out = path.join(root, "docs", "img");
const chrome =
  process.env.CHROME ||
  ["C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe", "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe"].find(existsSync);
if (!chrome) throw new Error("Chrome ou Edge introuvable : définis CHROME=<chemin>");

const shots = [
  { name: "home", page: "home" },
  { name: "verdicts", page: "analyse", extra: "&autoscan=1&scroll=.items.grid-5" },
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
  mkdirSync(out, { recursive: true });
  const server = spawn("npx", ["vite", "preview", "--outDir", "dist-demo", "--port", String(port), "--strictPort", "--host", "127.0.0.1"], {
    cwd: root,
    shell: true,
    stdio: "ignore",
  });
  try {
    await waitForServer();
    for (const s of shots) {
      const file = path.join(out, `${s.name}.png`);
      execFileSync(chrome, [
        "--headless=new",
        "--disable-gpu",
        "--hide-scrollbars",
        "--force-device-scale-factor=1",
        "--window-size=1280,820",
        "--virtual-time-budget=8000",
        `--screenshot=${file}`,
        `http://127.0.0.1:${port}/?page=${s.page}&theme=dark&lang=en${s.extra || ""}`,
      ]);
      console.log("ok", path.relative(root, file));
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
