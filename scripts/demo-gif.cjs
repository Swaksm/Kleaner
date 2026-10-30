// GIF du README : `npm run demo:build && node scripts/demo-gif.cjs` (ffmpeg dans le PATH, ou FFMPEG=<chemin>).
// Pilote l'interface en mode « demo » (données fictives) dans Chrome sans fenêtre par le protocole DevTools,
// capture des images pendant un petit parcours (analyse, verdict, nettoyage) puis les assemble avec ffmpeg.
// CHROME=<chemin de chrome.exe> si Chrome n'est pas à l'emplacement habituel.
const { spawn, execFileSync } = require("node:child_process");
const { existsSync, mkdirSync, rmSync, writeFileSync } = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.join(__dirname, "..");
const out = path.join(root, "docs", "img", "demo.gif");
const frames = path.join(os.tmpdir(), `kleaner-gif-${process.pid}`);
const chrome =
  process.env.CHROME ||
  ["C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe", "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe"].find(existsSync);
if (!chrome) throw new Error("Chrome ou Edge introuvable : définis CHROME=<chemin>");
const ffmpeg = process.env.FFMPEG || "ffmpeg";
const [port, debug] = [4174, 9333];
const [width, height] = [1280, 800];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitFor(url) {
  for (let i = 0; i < 60; i++) {
    try {
      const r = await fetch(url);
      if (r.ok) return r;
    } catch {}
    await sleep(500);
  }
  throw new Error(`${url} ne répond pas`);
}

/** Connexion DevTools minimale : `send(méthode, paramètres)` renvoie le résultat. */
async function connect(wsUrl) {
  const ws = new WebSocket(wsUrl);
  await new Promise((ok, ko) => ((ws.onopen = ok), (ws.onerror = ko)));
  let id = 0;
  const pending = new Map();
  ws.onmessage = (m) => {
    const msg = JSON.parse(m.data);
    if (msg.id && pending.has(msg.id)) {
      const { ok, ko } = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? ko(new Error(msg.error.message)) : ok(msg.result);
    }
  };
  const send = (method, params = {}) =>
    new Promise((ok, ko) => {
      pending.set(++id, { ok, ko });
      ws.send(JSON.stringify({ id, method, params }));
    });
  return { send, close: () => ws.close() };
}

// Curseur dessiné dans la page (Chrome sans fenêtre n'en a pas). En « popover » pour passer au-dessus des fenêtres modales.
const CURSOR = `(() => {
  const c = document.createElement("div");
  c.id = "gif-cursor";
  c.popover = "manual";
  c.style.cssText = "position:fixed;inset:auto;margin:0;padding:0;border:2px solid #111;width:18px;height:18px;border-radius:50%;" +
    "background:rgba(255,255,255,.9);box-shadow:0 0 0 4px rgba(255,255,255,.25);left:640px;top:420px;" +
    "transition:left .6s ease,top .6s ease,transform .15s;pointer-events:none;overflow:visible";
  document.body.appendChild(c);
  c.showPopover();
})()`;

/** Amène le curseur sur l'élément (sélecteur + texte facultatif), puis clique. */
const clickJs = (selector, text) => `(async () => {
  const el = [...document.querySelectorAll(${JSON.stringify(selector)})].find((e) => ${JSON.stringify(text || "")} === "" || e.textContent.includes(${JSON.stringify(text || "")}));
  if (!el) throw new Error("introuvable : ${selector} ${text || ""}");
  el.scrollIntoView({ block: "center", behavior: "smooth" });
  await new Promise((r) => setTimeout(r, 500));
  const c = document.getElementById("gif-cursor");
  c.hidePopover(); c.showPopover();
  const r = el.getBoundingClientRect();
  c.style.left = (r.left + r.width / 2 - 9) + "px";
  c.style.top = (r.top + r.height / 2 - 9) + "px";
  await new Promise((r) => setTimeout(r, 750));
  c.style.transform = "scale(.7)";
  await new Promise((r) => setTimeout(r, 150));
  c.style.transform = "";
  el.click();
})()`;

(async () => {
  const server = spawn("npx", ["vite", "preview", "--outDir", "dist-demo", "--port", String(port), "--strictPort", "--host", "127.0.0.1"], {
    cwd: root,
    shell: true,
    stdio: "ignore",
  });
  const profile = path.join(os.tmpdir(), `kleaner-gif-profile-${process.pid}`);
  const browser = spawn(chrome, [
    "--headless=new",
    "--disable-gpu",
    "--hide-scrollbars",
    `--remote-debugging-port=${debug}`,
    `--user-data-dir=${profile}`,
    `--window-size=${width},${height}`,
    "about:blank",
  ]);
  let cdp;
  try {
    await waitFor(`http://127.0.0.1:${port}/`);
    const targets = await (await waitFor(`http://127.0.0.1:${debug}/json/list`)).json();
    cdp = await connect(targets.find((t) => t.type === "page").webSocketDebuggerUrl);
    const { send } = cdp;
    await send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
    const run = async (expression) => {
      const r = await send("Runtime.evaluate", { expression, awaitPromise: true });
      if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description || r.exceptionDetails.text);
      return r.result.value;
    };
    await send("Page.navigate", { url: `http://127.0.0.1:${port}/?page=home&theme=dark&lang=en` });
    await sleep(2000);
    await run(CURSOR);

    // Capture continue pendant le parcours : chaque image garde sa vraie durée.
    rmSync(frames, { recursive: true, force: true });
    mkdirSync(frames, { recursive: true });
    const shots = [];
    let recording = true;
    const recorder = (async () => {
      while (recording) {
        const t = Date.now();
        const { data } = await send("Page.captureScreenshot", { format: "png" });
        const file = path.join(frames, `f${String(shots.length).padStart(4, "0")}.png`);
        writeFileSync(file, Buffer.from(data, "base64"));
        shots.push({ file, t });
        await sleep(Math.max(0, 100 - (Date.now() - t)));
      }
    })();

    await sleep(1500);
    await run(clickJs("button[data-tour=scan]")); // Accueil : analyser
    await sleep(3000);
    await run(clickJs(".verdict-tag")); // premier verdict de la liste
    await sleep(3500);
    await run(clickJs("dialog[open] button", "Close")); // fermer la fiche
    await sleep(800);
    await run(clickJs(".sidebar button", "Cleanup"));
    await sleep(2000);
    await run(clickJs(".cleanup-bar button", "Tick risk-free items"));
    await sleep(1500);
    await run(clickJs(".cleanup-bar button", "Clean selection"));
    await sleep(2000);
    await run(clickJs("dialog[open] button", "Clean selection"));
    // Les estimations se rechargent après le nettoyage : ramener la carte « libéré » à l'écran ensuite.
    await sleep(1200);
    await run(`document.querySelector(".done-card").scrollIntoView({ block: "center", behavior: "smooth" })`);
    await sleep(3500);
    recording = false;
    await recorder;

    // Liste ffmpeg : chaque image dure jusqu'à la suivante.
    const list = shots
      .map((s, i) => `file '${s.file.replace(/\\/g, "/")}'\nduration ${((shots[i + 1]?.t ?? s.t + 1500) - s.t) / 1000}`)
      .join("\n");
    const listFile = path.join(frames, "list.txt");
    writeFileSync(listFile, `${list}\nfile '${shots[shots.length - 1].file.replace(/\\/g, "/")}'\n`);
    execFileSync(ffmpeg, [
      "-y",
      "-loglevel", "error",
      "-f", "concat",
      "-safe", "0",
      "-i", listFile,
      "-vf", "fps=12,scale=900:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle",
      out,
    ]);
    console.log(`${path.relative(root, out)} (${shots.length} images)`);
  } finally {
    cdp?.close();
    browser.kill();
    server.kill();
    rmSync(frames, { recursive: true, force: true });
    rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 300 });
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
