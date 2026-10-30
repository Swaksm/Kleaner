// Test de bout en bout : pilote le vrai kleaner.exe via tauri-driver + msedgedriver (WebDriver).
// Usage : node e2e/run.mjs <chemin de kleaner.exe> <chemin de msedgedriver.exe>
// KLEANER_E2E_DESTRUCTIVE=1 : lance aussi un vrai nettoyage et crée une photo de test (CI uniquement).
import { spawn } from "node:child_process";
import { mkdirSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { remote } from "webdriverio";

const [exe, edgedriver] = process.argv.slice(2);
if (!exe || !edgedriver) throw new Error("usage: node e2e/run.mjs <kleaner.exe> <msedgedriver.exe>");
const destructive = process.env.KLEANER_E2E_DESTRUCTIVE === "1";

const driver = spawn("tauri-driver", ["--native-driver", edgedriver], { stdio: ["ignore", "inherit", "inherit"] });
await new Promise((r) => setTimeout(r, 2000));

let failures = 0;
const step = async (name, fn) => {
  const t = Date.now();
  try {
    await fn();
    console.log(`  ok   ${name} (${Date.now() - t} ms)`);
  } catch (e) {
    failures++;
    console.log(`  FAIL ${name}: ${e.message}`);
  }
};
const assert = (cond, msg) => {
  if (!cond) throw new Error(msg);
};

// Photo de test pour la visionneuse (PNG 1×1 valide).
const picture = join(process.env.USERPROFILE, "Pictures", "kleaner-e2e");
if (destructive) {
  mkdirSync(picture, { recursive: true });
  const png = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==",
    "base64",
  );
  // Assez gros pour la liste des photos (≥ 50 Ko) : un PNG valide suivi de remplissage.
  writeFileSync(join(picture, "e2e.png"), Buffer.concat([png, Buffer.alloc(60 * 1024)]));
}

const b = await remote({
  hostname: "127.0.0.1",
  port: 4444,
  logLevel: "warn",
  capabilities: { "tauri:options": { application: exe } },
});
const nav = async (label) => (await (await b.$(".sidebar")).$(`button=${label}`)).click();
const inContent = async (sel) => (await b.$(".content")).$(sel);
const h1 = async () => (await b.$(".content h1")).getText();

try {
  await step("avertissement du premier lancement", async () => {
    const box = await b.$(".disclaimer input[type=checkbox]");
    if (await box.isExisting()) {
      await box.click();
      await (await b.$("button=Continue")).click();
    }
    await (await b.$(".sidebar")).waitForExist({ timeout: 10_000 }).catch(() => {});
  });

  await step("assistant du premier lancement (choix par défaut)", async () => {
    // « .setup button=Finish » n'est pas un sélecteur valide pour WebDriver : chercher le bouton dans l'assistant.
    const setup = await b.$(".setup");
    if (await setup.isExisting()) await (await setup.$("button=Finish")).click();
    await (await b.$(".sidebar")).waitForExist({ timeout: 10_000 });
    // La visite guidée proposée à la fin de l'assistant rend l'appli inerte : la passer.
    const skip = await b.$("button=Skip the tour");
    if (await skip.waitForExist({ timeout: 5_000 }).catch(() => false)) await skip.click();
  });

  await step("accueil en anglais par défaut", async () => {
    assert((await h1()) === "Home", `h1 = ${await h1()}`);
  });

  await step("programmes : liste lue dans le registre", async () => {
    await nav("Programs");
    await b.waitUntil(async () => (await b.$$(".items.programs li")).length > 1, { timeout: 20_000, timeoutMsg: "aucun programme" });
  });

  await step("restes de programmes : recherche", async () => {
    await (await b.$("button=Find leftovers")).click();
    await b.waitUntil(async () => !(await (await b.$(".loading")).isExisting()), { timeout: 60_000 });
  });

  await step("nettoyage : estimations et simulation", async () => {
    await nav("Cleanup");
    await b.waitUntil(async () => !(await (await b.$("button=Estimating…")).isExisting()), { timeout: 120_000 });
    await (await b.$("button=Tick risk-free items")).click();
    const preview = await b.$("button=Preview the exact list");
    if (await preview.isEnabled()) {
      await preview.click();
      await (await b.$("dialog[open] .preview-entry")).waitForExist({ timeout: 60_000 });
      await (await (await b.$("dialog[open]")).$("button=Close")).click();
    }
  });

  if (destructive) {
    await step("nettoyage réel + journal", async () => {
      const run = await b.$("button=Clean selection");
      if (!(await run.isEnabled())) return; // rien à nettoyer sur cette machine
      await run.click();
      await (await (await b.$("dialog[open]")).$("button=Clean selection")).click();
      await (await b.$("h2=Result")).waitForExist({ timeout: 120_000 });
      await nav("More tools");
      await nav("Log");
      await b.waitUntil(async () => (await b.$$(".card strong")).some(Boolean), { timeout: 10_000 });
      const text = await (await b.$(".content")).getText();
      assert(text.includes("clean:"), "pas d'entrée clean: dans le journal");
    });
  }

  await step("analyse de C:\\ complète", async () => {
    await nav("Scan");
    await (await inContent("button=Scan")).click();
    await (await b.$(".donut")).waitForExist({ timeout: 600_000 });
    await b.waitUntil(async () => (await b.$$(".verdict-tag:not(.muted)")).length > 0, { timeout: 120_000, timeoutMsg: "pas de verdicts" });
  });

  if (destructive) {
    await step("visionneuse : la photo de test s'affiche", async () => {
      await nav("Photos & videos");
      await (await b.$("button*=Photos")).click();
      await (await b.$("input[type=search]")).setValue("kleaner-e2e");
      await (await (await b.$(".media-list")).$("button=View")).click();
      const img = await b.$("dialog[open] .viewer-stage img");
      await img.waitForExist({ timeout: 10_000 });
      await b.waitUntil(async () => (await b.execute((el) => el.naturalWidth, img)) > 0, { timeout: 10_000, timeoutMsg: "image non chargée" });
      await b.keys("Escape");
    });
  }

  await step("accueil : recommandations après l'analyse", async () => {
    await nav("Home");
    await (await b.$(".recs li")).waitForExist({ timeout: 120_000 });
  });

  await step("mises à jour : machine et applis (winget)", async () => {
    await nav("Updates & drivers");
    await b.waitUntil(async () => (await (await b.$(".content")).getText()).includes("BIOS"), { timeout: 30_000, timeoutMsg: "infos machine absentes" });
    await (await b.$("button=Check app updates")).click();
    await b.waitUntil(async () => !(await (await b.$(".loading")).isExisting()), { timeout: 180_000, timeoutMsg: "winget trop long" });
  });

  await step("démarrage : programmes lus", async () => {
    await nav("Startup");
    await b.waitUntil(async () => (await b.$$(".items.grid-startup li")).length > 0, { timeout: 90_000, timeoutMsg: "aucun programme au démarrage" });
  });

  await step("santé : bilan en lecture seule", async () => {
    await nav("Health & security");
    await b.waitUntil(async () => (await b.$$(".items.grid-health li")).length > 3, { timeout: 120_000, timeoutMsg: "bilan vide" });
  });

  await step("raccourcis cassés et entrées fantômes : recherches", async () => {
    const link = await (await b.$(".sidebar")).$("button=Broken shortcuts");
    // Le bouton du groupe se termine par une flèche (▸) : correspondance partielle.
    if (!(await link.isExisting())) await (await (await b.$(".sidebar")).$("button*=More tools")).click();
    await nav("Broken shortcuts");
    await (await inContent("button=Find broken shortcuts")).click();
    await b.waitUntil(async () => !(await (await b.$(".loading")).isExisting()), { timeout: 120_000, timeoutMsg: "recherche trop longue" });
    await nav("Programs");
    await (await inContent("button=Find ghost entries")).click();
    await (await b.$("p=No ghost entry found.")).waitForExist({ timeout: 60_000 }).catch(() => {});
  });

  await step("applis préinstallées : liste en lecture seule", async () => {
    await nav("Programs");
    await (await inContent("button=Find preinstalled apps")).click();
    await b.waitUntil(
      async () => (await b.$$(".grid-preinstalled li")).length > 0 || (await (await b.$("p=No app from the list is installed.")).isExisting()),
      { timeout: 60_000, timeoutMsg: "ni liste ni message « aucune appli »" },
    );
  });

  await step("conseils : recherche en lecture seule", async () => {
    await nav("Tips");
    await (await inContent("button=Find tips")).click();
    await b.waitUntil(async () => !(await (await b.$(".loading")).isExisting()), { timeout: 60_000, timeoutMsg: "recherche trop longue" });
    await b.waitUntil(async () => (await b.$$(".content .card")).length > 1 || (await (await b.$("p=Nothing to report.")).isExisting()), {
      timeout: 10_000,
      timeoutMsg: "ni conseil ni message « rien à signaler »",
    });
  });

  await step("paramètres : passage en français", async () => {
    await (await b.$(".sidebar button[aria-label=Settings]")).click();
    await b.waitUntil(async () => (await h1()) === "Settings", { timeout: 10_000, timeoutMsg: "page Paramètres absente" });
    // Le menu des langues, pas un autre menu de la page.
    const language = await b.$("select:has(option[value=fr])");
    await language.selectByAttribute("value", "fr");
    assert((await h1()) === "Paramètres", `h1 = ${await h1()}`);
    await language.selectByAttribute("value", "en");
  });
} finally {
  await b.deleteSession().catch(() => {});
  driver.kill();
  if (destructive) rmSync(picture, { recursive: true, force: true });
}

console.log(failures === 0 ? "\nE2E: all steps passed" : `\nE2E: ${failures} step(s) failed`);
process.exit(failures === 0 ? 0 : 1);
