// Données fictives pour les captures d'écran du README (`npm run demo:shots`). Jamais incluses dans l'exe :
// seul le mode « demo » de Vite remplace les appels à Tauri par ce module.
import cleanersJson from "../../src-tauri/cleaners.json";
import knowledgeJson from "../../src-tauri/knowledge.json";
import type {
  Boot,
  Cleaner,
  CleanReport,
  DriveInfo,
  Explanation,
  HealthItem,
  Item,
  Leftover,
  ScanResult,
  StartupItem,
  Verdict,
} from "../lib/api";

const GB = 1024 ** 3;
const MB = 1024 ** 2;
const DAY = 86_400;
const now = () => Math.floor(Date.now() / 1000);
const item = (path: string, size: number, daysAgo: number): Item => ({ path, size: Math.round(size), modified: now() - daysAgo * DAY });

const HOME = "C:\\Users\\alex";
const LOCAL = `${HOME}\\AppData\\Local`;

export const drives = (): DriveInfo[] => [
  { root: "C:\\", kind: "fixed", total: 476 * GB, free: 112 * GB },
  { root: "D:\\", kind: "fixed", total: 1000 * GB, free: 611 * GB },
];

// ---------- Analyse ----------

export const scanResult = (): ScanResult => ({
  roots: ["C:\\", "D:\\"],
  bytes: 753 * GB,
  files: 1_284_330,
  denied: 0,
  cancelled: false,
  full_access: false,
  fast: false,
  duration_ms: 41_000,
  drives: drives().map(({ root, total, free }) => ({ root, total, free })),
  top_files: [
    item(`${LOCAL}\\Docker\\wsl\\disk\\docker_data.vhdx`, 41.2 * GB, 0),
    item("D:\\Games\\SteamLibrary\\steamapps\\common\\Big Game\\data\\main.pak", 38.4 * GB, 40),
    item(`${HOME}\\.ollama\\models\\blobs\\sha256-6a0746a1ec1aef3e7ec53868f220ff6e389f6f8ef87a01d77c96807de94ca2aa`, 19.9 * GB, 12),
    item(`${HOME}\\.cache\\huggingface\\hub\\models--meta-llama--Llama-3.1-8B\\blobs\\a1b2c3d4e5f6`, 14.9 * GB, 75),
    item(`${LOCAL}\\Android\\Sdk\\system-images\\android-34\\google_apis\\x86_64\\system.img`, 6.3 * GB, 210),
    item("C:\\pagefile.sys", 6.3 * GB, 0),
    item(`${HOME}\\Videos\\Holiday 2024.mp4`, 6.8 * GB, 300),
    item(`${HOME}\\Downloads\\ubuntu-24.04-desktop-amd64.iso`, 5.9 * GB, 190),
    item(`${HOME}\\Documents\\Backups\\laptop-2023.zip`, 4.4 * GB, 600),
    item("C:\\Windows\\MEMORY.DMP", 3.5 * GB, 33),
    item(`${LOCAL}\\Temp\\installer-cache.tmp`, 2.2 * GB, 90),
    item(`${HOME}\\Tools\\mystery-tool\\data.bin`, 1.8 * GB, 160),
    item(`${HOME}\\Projects\\old-site\\node_modules\\@next\\swc-win32-x64-msvc\\next-swc.win32-x64-msvc.node`, 0.17 * GB, 420),
  ],
  top_dirs: [
    item("D:\\Games\\SteamLibrary", 318 * GB, 3),
    item(`${LOCAL}\\Docker`, 41.2 * GB, 0),
    item(`${HOME}\\.ollama`, 19.9 * GB, 12),
    item(`${HOME}\\.cache`, 15.2 * GB, 75),
    item(`${LOCAL}\\Android`, 12.4 * GB, 210),
    item(`${HOME}\\Projects`, 11.1 * GB, 4),
    item(`${HOME}\\Videos`, 7.2 * GB, 300),
    item("C:\\Windows\\WinSxS", 9.8 * GB, 2),
  ],
  videos: [item(`${HOME}\\Videos\\Holiday 2024.mp4`, 6.8 * GB, 300)],
  images: [item(`${HOME}\\Pictures\\IMG_2041.jpg`, 4.1 * MB, 120)],
  artifacts: [
    { path: `${HOME}\\Projects\\old-site\\node_modules`, kind: "node", size: 1.4 * GB, project: `${HOME}\\Projects\\old-site`, project_modified: now() - 420 * DAY },
    { path: `${HOME}\\Projects\\ml-demo\\.venv`, kind: "python", size: 4.9 * GB, project: `${HOME}\\Projects\\ml-demo`, project_modified: now() - 260 * DAY },
    { path: `${HOME}\\Projects\\game-jam\\target`, kind: "rust", size: 3.2 * GB, project: `${HOME}\\Projects\\game-jam`, project_modified: now() - 150 * DAY },
  ],
  tree: [
    item("C:\\", 364 * GB, 0),
    item("D:\\", 389 * GB, 0),
    item("C:\\Users", 148 * GB, 0),
    item("C:\\Windows", 26 * GB, 2),
    item("C:\\Program Files", 33 * GB, 10),
    item("C:\\Program Files (x86)", 14 * GB, 10),
    item("C:\\ProgramData", 9 * GB, 5),
    item("C:\\Users\\alex", 148 * GB, 0),
    item(`${HOME}\\AppData`, 88 * GB, 0),
    item(`${HOME}\\.ollama`, 20 * GB, 12),
    item(`${HOME}\\.cache`, 15 * GB, 75),
    item(`${HOME}\\Projects`, 11 * GB, 4),
    item(`${HOME}\\Videos`, 7 * GB, 300),
    item(`${HOME}\\Downloads`, 6 * GB, 20),
    item(`${HOME}\\AppData\\Local`, 80 * GB, 0),
    item(`${HOME}\\AppData\\Roaming`, 8 * GB, 0),
    item(`${LOCAL}\\Docker`, 41 * GB, 0),
    item(`${LOCAL}\\Android`, 12 * GB, 210),
    item(`${LOCAL}\\Programs`, 9 * GB, 20),
    item(`${LOCAL}\\Temp`, 3 * GB, 0),
    item("D:\\Games", 318 * GB, 3),
    item("D:\\Videos", 41 * GB, 60),
    item("D:\\Work", 18 * GB, 7),
    item("D:\\Games\\SteamLibrary", 318 * GB, 3),
  ],
  categories: [
    { category: "videos", bytes: 61 * GB, files: 410 },
    { category: "archives", bytes: 58 * GB, files: 1_900 },
    { category: "images", bytes: 22 * GB, files: 38_000 },
    { category: "installers", bytes: 9 * GB, files: 70 },
    { category: "audio", bytes: 7 * GB, files: 5_200 },
    { category: "other", bytes: 596 * GB, files: 1_238_750 },
  ],
});

// ---------- Verdicts : les vraies règles de knowledge.json ----------

type Rule = {
  id: string;
  match: string[];
  generic?: boolean;
  whole?: boolean;
  verdict: string;
  what: { en: string; fr: string };
  consequence: { en: string; fr: string };
  howto: { en: string; fr: string };
};

const ENV: Record<string, string> = {
  USERPROFILE: HOME,
  LOCALAPPDATA: LOCAL,
  APPDATA: `${HOME}\\AppData\\Roaming`,
  ProgramData: "C:\\ProgramData",
  ProgramFiles: "C:\\Program Files",
  "ProgramFiles(x86)": "C:\\Program Files (x86)",
  SystemRoot: "C:\\Windows",
  WinDir: "C:\\Windows",
  SystemDrive: "C:",
  TEMP: `${LOCAL}\\Temp`,
};

const expand = (p: string) => p.replace(/%([^%]+)%/g, (_, v: string) => ENV[v] ?? `%${v}%`);

/** Glob de Kleaner (`*`, `**`, `**\`, `?`) → expression régulière ; chaque `*` est capturé pour {1}, {2}… */
function toRegex(pattern: string): RegExp {
  let re = "";
  for (let i = 0; i < pattern.length; i++) {
    const c = pattern[i];
    if (c === "*" && pattern[i + 1] === "*") {
      if (pattern[i + 2] === "\\") {
        re += "(?:.*\\\\)?";
        i += 2;
      } else {
        re += ".*";
        i += 1;
      }
    } else if (c === "*") re += "([^\\\\]*)";
    else if (c === "?") re += "[^\\\\]";
    else re += c.replace(/[.+^${}()|[\]\\]/g, "\\$&");
  }
  return new RegExp(`^${re}$`, "i");
}

const rules = knowledgeJson as unknown as Rule[];
const fill = (s: string, caps: string[]) => s.replace(/\{(\d)\}/g, (_, n: string) => caps[+n - 1] ?? "");

export function explain(path: string): Explanation {
  const parts = path.split("\\");
  const prefixes = parts.map((_, i) => parts.slice(0, i + 1).join("\\"));
  for (const generic of [false, true]) {
    for (const r of rules.filter((x) => !!x.generic === generic)) {
      for (const pat of r.match) {
        const re = toRegex(expand(pat));
        for (const prefix of prefixes) {
          const m = re.exec(prefix);
          if (!m) continue;
          const caps = m.slice(1);
          const base = {
            path,
            source: "rule" as const,
            rule: r.id,
            cleaner: null,
          };
          if (r.whole && prefix.length < path.length) {
            return {
              ...base,
              verdict: "breaks",
              what: { en: `Inside ${prefix}. ${fill(r.what.en, caps)}`, fr: `Dans ${prefix}. ${fill(r.what.fr, caps)}` },
              consequence: { en: "Deleting this file alone breaks the whole folder.", fr: "Supprimer ce fichier seul casse tout le dossier." },
              howto: { en: `Only the whole folder can go: ${prefix}. ${fill(r.howto.en, caps)}`, fr: `Seul le dossier entier peut partir : ${prefix}. ${fill(r.howto.fr, caps)}` },
              locked: true,
            };
          }
          return {
            ...base,
            verdict: r.verdict as Verdict,
            what: { en: fill(r.what.en, caps), fr: fill(r.what.fr, caps) },
            consequence: { en: fill(r.consequence.en, caps), fr: fill(r.consequence.fr, caps) },
            howto: { en: fill(r.howto.en, caps), fr: fill(r.howto.fr, caps) },
            locked: false,
          };
        }
      }
    }
  }
  return {
    path,
    verdict: "unknown",
    source: "unknown",
    rule: null,
    cleaner: null,
    what: { en: "Kleaner doesn’t know this item.", fr: "Kleaner ne connaît pas cet élément." },
    consequence: { en: "Unknown: don’t delete it unless you know what it is.", fr: "Inconnu : ne le supprime pas sans savoir ce que c’est." },
    howto: {
      en: "Open it in Explorer to see what’s inside, search the folder name online, or use « Report for an AI ».",
      fr: "Ouvre-le dans l’Explorateur pour voir le contenu, cherche le nom du dossier en ligne, ou utilise « Rapport pour une IA ».",
    },
    locked: false,
  };
}

// ---------- Nettoyage ----------

const SIZES: Record<string, number> = {
  "user-temp": 2.4 * GB,
  thumbnails: 380 * MB,
  "inet-cache": 210 * MB,
  "crash-dumps": 640 * MB,
  "shader-cache": 1.1 * GB,
  "browser-chrome": 1.9 * GB,
  "browser-edge": 640 * MB,
  "app-discord": 420 * MB,
  "app-vscode": 310 * MB,
  "dev-npm": 3.2 * GB,
  "dev-pip": 2.9 * GB,
  "dev-gradle": 4.8 * GB,
  "dev-cargo": 1.7 * GB,
  "game-steam": 2.2 * GB,
  "recycle-bin": 560 * MB,
  "windows-update-cache": 1.0 * GB,
};

export const cleaners = (): Cleaner[] =>
  (cleanersJson as unknown as Partial<Cleaner>[]).map((c) => ({ only: [], flat: false, close: [], command: null, admin: false, ...c }) as Cleaner);

export const estimates = (): CleanReport[] =>
  cleaners()
    .filter((c) => !c.admin)
    .map((c) => ({
      id: c.id,
      files: SIZES[c.id] ? Math.round(SIZES[c.id] / (300 * 1024)) : 0,
      bytes: Math.round(SIZES[c.id] ?? 0),
      skipped: 0,
      blocked_by: [],
      output: null,
      error: null,
    }));

export const leftovers = (): Leftover[] => [
  { path: `${LOCAL}\\Arduino15`, name: "Arduino15", place: "appdata", size: 8.1 * GB, modified: now() - 200 * DAY, trashable: true, warning: null },
  { path: `${HOME}\\AppData\\Roaming\\OldPlayer`, name: "OldPlayer", place: "appdata", size: 640 * MB, modified: now() - 380 * DAY, trashable: true, warning: null },
];

// ---------- Démarrage ----------

const startup = (name: string, command: string, o: Partial<StartupItem> = {}): StartupItem => ({
  id: `reg|HKCU|${name}`,
  name,
  command,
  source: "registry",
  machine: false,
  enabled: true,
  can_toggle: true,
  keep: false,
  slow_ms: 0,
  missing: false,
  ...o,
});

export const startupView = (): { items: StartupItem[]; boots: Boot[] } => ({
  items: [
    startup("Docker Desktop", '"C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe"', { slow_ms: 14_200 }),
    startup("Steam", '"C:\\Program Files (x86)\\Steam\\steam.exe" -silent', { slow_ms: 4_600 }),
    startup("OneDrive", `"${LOCAL}\\Microsoft\\OneDrive\\OneDrive.exe" /background`, { slow_ms: 5_100 }),
    startup("Discord", `"${LOCAL}\\Discord\\Update.exe" --processStart Discord.exe`),
    startup("Spotify", `"${HOME}\\AppData\\Roaming\\Spotify\\Spotify.exe" /minimized`),
    startup("Corsair iCUE5 Software", '"C:\\Program Files\\Corsair\\Corsair iCUE5 Software\\iCUE Launcher.exe" --autorun', { machine: true, id: "reg|HKLM|Corsair" }),
    startup("RtkAudUService", '"C:\\Windows\\System32\\DriverStore\\FileRepository\\realtekservice\\RtkAudUService64.exe" -background', { keep: true, machine: true, id: "reg|HKLM|Rtk" }),
    startup("SecurityHealth", "C:\\Windows\\system32\\SecurityHealthSystray.exe", { keep: true, machine: true, id: "reg|HKLM|SecurityHealth" }),
    startup("Opera GX Browser Assistant", `"${LOCAL}\\Programs\\Opera GX\\assistant\\browser_assistant.exe"`, { source: "task", id: "task|\\|Opera" }),
    startup("Epic Games Launcher", '"C:\\Program Files (x86)\\Epic Games\\Launcher\\Portal\\Binaries\\Win64\\EpicGamesLauncher.exe" -silent', { enabled: false }),
    startup("Adobe Acrobat Synchronizer", '"C:\\Program Files\\Adobe\\Acrobat DC\\Acrobat\\AdobeCollabSync.exe"', { enabled: false }),
    startup("NVIDIA LocalSystem Container", '"C:\\Program Files\\NVIDIA Corporation\\NvContainer\\nvcontainer.exe" -s NvContainerLocalSystem', {
      source: "service",
      can_toggle: false,
      machine: true,
      keep: true,
      id: "svc|NvContainer",
    }),
  ],
  boots: [
    { time: now() - 1 * DAY, boot_ms: 31_200 },
    { time: now() - 2 * DAY, boot_ms: 28_900 },
    { time: now() - 4 * DAY, boot_ms: 34_500 },
  ],
});

// ---------- Santé et sécurité ----------

const h = (id: string, group: HealthItem["group"], status: HealthItem["status"], en: [string, string], fr: [string, string], fix: string | null = null): HealthItem => ({
  id,
  group,
  status,
  title: { en: en[0], fr: fr[0] },
  detail: { en: en[1], fr: fr[1] },
  fix,
});

export const health = (): HealthItem[] => [
  h("disk", "disk", "ok", ["Disk KXG80ZNV512G (SSD)", "Health: Healthy, wear 6%, 38 °C, 4 120 h powered on."], ["Disque KXG80ZNV512G (SSD)", "État : Healthy, usure 6 %, 38 °C, 4 120 h de fonctionnement."]),
  h("space", "disk", "ok", ["Free space on C:\\", "112.0 GB free (23%). Windows slows down and cannot update below about 10%."], ["Espace libre sur C:\\", "112,0 Go libres (23 %). Sous 10 % environ, Windows ralentit et ne peut plus se mettre à jour."]),
  h("bsod", "system", "ok", ["Blue screens (30 days)", "None in the last 30 days."], ["Écrans bleus (30 jours)", "Aucun ces 30 derniers jours."]),
  h("app-crashes", "system", "warn", ["Application crashes (30 days)", "Most frequent: OldPlayer.exe ×14, Overlay.exe ×3. A program that crashes often should be updated or reinstalled."], ["Plantages d’applications (30 jours)", "Les plus fréquents : OldPlayer.exe ×14, Overlay.exe ×3. Un programme qui plante souvent est à mettre à jour ou à réinstaller."]),
  h("reboot", "system", "warn", ["Restart pending", "Windows is waiting for a restart to finish installing updates."], ["Redémarrage en attente", "Windows attend un redémarrage pour terminer l’installation de mises à jour."], "ms-settings:windowsupdate"),
  h("uptime", "system", "ok", ["Time since last restart", "2 days. A restart finishes pending updates and frees leaked memory."], ["Durée depuis le dernier redémarrage", "2 jours. Un redémarrage termine les mises à jour en attente et libère la mémoire."]),
  h("memory", "system", "ok", ["Memory in use", "62% of 32 GB right now. Close heavy programs if it stays above 90%."], ["Mémoire utilisée", "62 % de 32 Go en ce moment. Ferme les programmes lourds si ça reste au-dessus de 90 %."]),
  h("power", "system", "ok", ["Power plan", "Balanced."], ["Mode d’alimentation", "Utilisation normale."]),
  h("antivirus", "security", "ok", ["Antivirus", "Windows Defender: active and up to date."], ["Antivirus", "Windows Defender : actif et à jour."]),
  h("firewall", "security", "ok", ["Firewall", "On for every network profile."], ["Pare-feu", "Activé pour tous les types de réseau."]),
  h("uac", "security", "ok", ["User Account Control (UAC)", "On: Windows asks before a program changes the system."], ["Contrôle de compte d’utilisateur (UAC)", "Activé : Windows demande avant qu’un programme modifie le système."]),
  h("secureboot", "security", "warn", ["Secure Boot", "Off. It is turned on in the PC firmware (UEFI) settings."], ["Démarrage sécurisé (Secure Boot)", "Désactivé. Il s’active dans les réglages du micrologiciel (UEFI) du PC."], "ms-settings:recovery"),
  h("bitlocker", "security", "info", ["Drive encryption (BitLocker)", "Off. Worth turning on on a laptop; keep your recovery key safe."], ["Chiffrement du disque (BitLocker)", "Désactivé. À activer sur un portable ; garde ta clé de récupération en lieu sûr."], "ms-settings:deviceencryption"),
  h("support", "security", "ok", ["Windows support", "Security updates until 2027-10-12 (362 days)."], ["Support de Windows", "Mises à jour de sécurité jusqu’au 2027-10-12 (362 jours)."]),
];
