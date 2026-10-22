// Seul point de contact entre l'interface et le Rust.
import { invoke } from "@tauri-apps/api/core";
import { friendlyError } from "./errors";
import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

/** `invoke` avec les erreurs connues traduites dans la langue de l'interface. */
function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(cmd, args).catch((e) => {
    throw friendlyError(String(e));
  });
}

export type JournalEntry = {
  ts: number;
  action: string;
  paths: string[];
  files: number;
  bytes: number;
  skipped: number;
  error: string | null;
  output: string | null;
};

export type JournalView = { path: string; entries: JournalEntry[] };

export type Category = "videos" | "images" | "audio" | "archives" | "installers" | "other";
export type Item = { size: number; path: string; modified: number };
export type ScanProgress = { files: number; bytes: number; denied: number };
export type ScanResult = {
  roots: string[];
  bytes: number;
  files: number;
  denied: number;
  cancelled: boolean;
  full_access: boolean;
  fast: boolean;
  duration_ms: number;
  top_files: Item[];
  top_dirs: Item[];
  videos: Item[];
  images: Item[];
  drives: { root: string; total: number; free: number }[];
  artifacts: Artifact[];
  tree: Item[];
  categories: { category: Category; bytes: number; files: number }[];
};
export type ActionReport = { files: number; bytes: number };

export const readJournal = (limit = 200) => call<JournalView>("read_journal", { limit });
export const removeKleanerData = () => call<void>("remove_kleaner_data");
export const revealInExplorer = (path: string) => revealItemInDir(path);

export const startScan = (roots?: string[]) => call<void>("start_scan", { roots });
export const cancelScan = () => call<void>("cancel_scan");
export const onScanProgress = (fn: (p: ScanProgress) => void) => listen<ScanProgress>("scan-progress", (e) => fn(e.payload));
export const onScanDone = (fn: (r: ScanResult) => void) => listen<ScanResult>("scan-done", (e) => fn(e.payload));

export const moveToTrash = (paths: string[]) => call<ActionReport>("move_to_trash", { paths });
export const saveReport = (content: string) => call<string>("save_report", { content });
export const previewMedia = (path: string) => call<string>("preview_media", { path });
export const openMedia = (path: string) => call<void>("open_media", { path });

export type Text = { fr: string; en: string };
export type Cleaner = {
  id: string;
  group: "windows" | "browsers" | "apps" | "dev" | "community" | "system" | "heavy";
  label: Text;
  what: Text;
  why: Text;
  effect: Text;
  paths: string[];
  only: string[];
  flat: boolean;
  admin: boolean;
  mode: "delete" | "command";
  level: "safe" | "heavy";
  close: string[];
  command: string | null;
};
export type CleanReport = {
  id: string;
  files: number;
  bytes: number;
  skipped: number;
  blocked_by: string[];
  output: string | null;
  error: string | null;
};
export type CleanersView = { elevated: boolean; cleaners: Cleaner[] };

export const listCleaners = () => call<CleanersView>("list_cleaners");
export const estimateCleaners = () => call<CleanReport[]>("estimate_cleaners");
export const runCleaners = (ids: string[]) => call<CleanReport[]>("run_cleaners", { ids });
export const relaunchAdmin = () => call<void>("relaunch_admin");
export const isElevated = () => call<boolean>("is_elevated");
export const micaEnabled = () => call<boolean>("mica");
export const autostartEnabled = () => call<boolean>("autostart_enabled");
export const setAutostart = (on: boolean) => call<void>("set_autostart", { on });

export type Program = {
  id: string;
  name: string;
  publisher: string;
  version: string;
  bytes: number;
  install_date: string;
  location: string;
};
export const listPrograms = () => call<Program[]>("list_programs");
export const uninstallProgram = (id: string) => call<void>("uninstall_program", { id });
export const programSizes = () => call<{ id: string; bytes: number; last_used: number }[]>("program_sizes");

export type Verdict = "safe" | "cleanup" | "uninstall" | "breaks" | "personal" | "system" | "unknown";
export type Explanation = {
  path: string;
  verdict: Verdict;
  source: "rule" | "cleaner" | "program" | "unknown";
  rule: string | null;
  cleaner: string | null;
  what: Text;
  consequence: Text;
  howto: Text;
  /** Mettre cet élément seul à la Corbeille casserait le dossier qui le contient. */
  locked: boolean;
};
/** Supprimer à la main est dangereux : confirmation renforcée. */
export const isCritical = (v: Verdict) => v === "breaks" || v === "system" || v === "uninstall";
export const explainPaths = (paths: string[]) => call<Explanation[]>("explain_paths", { paths });

export type DriveInfo = { root: string; kind: "fixed" | "removable" | "network" | "cdrom" | "other"; total: number; free: number };
export const listDrives = () => call<DriveInfo[]>("list_drives");

export type Artifact = {
  path: string;
  kind: "node" | "python" | "rust" | "gradle" | "build" | "dart" | "next" | "dotnet";
  size: number;
  project: string;
  project_modified: number;
};
export const deleteArtifacts = (paths: string[]) => call<CleanReport>("delete_artifacts", { paths });

export type DupGroup = { size: number; wasted: number; files: Item[] };
export const findDuplicates = () => call<DupGroup[]>("find_duplicates");
export const cancelDuplicates = () => call<void>("cancel_duplicates");
export const onDupProgress = (fn: (p: { done: number; total: number }) => void) =>
  listen<{ done: number; total: number }>("dup-progress", (e) => fn(e.payload));

export type PreviewView = {
  entries: { id: string; files: number; bytes: number; sample: Item[] }[];
  file: string;
};
export const previewCleaners = (ids: string[]) => call<PreviewView>("preview_cleaners", { ids });

export type Leftover = {
  path: string;
  name: string;
  place: "appdata" | "programs" | "profile" | "programfiles" | "programdata";
  size: number;
  modified: number;
  trashable: boolean;
  /** « android » : le SDK ou les émulateurs servent peut-être à d'autres outils (Flutter, Unity…). */
  warning: "android" | null;
};
export const findLeftovers = () => call<Leftover[]>("find_leftovers");
export const trashLeftovers = (paths: string[]) => call<ActionReport>("trash_leftovers", { paths });

export const restoreTrashed = (paths: string[]) => call<ActionReport>("restore_trashed", { paths });
export const bugReport = () => call<string>("bug_report");

export type CloudFile = { path: string; size: number; modified: number; pinned: boolean; provider: "OneDrive" | "Dropbox" };
export const onedriveRoots = () => call<string[]>("onedrive_roots");
export const onedriveFiles = () => call<CloudFile[]>("onedrive_files");
export const onedriveFree = (paths: string[]) => call<ActionReport>("onedrive_free", { paths });

export type Gpu = { name: string; vendor: string; driver_version: string; driver_date: string };
export type Disk = { name: string; media: string; health: string; size: number };
export type SystemInfo = {
  os_name: string;
  os_release: string;
  os_build: number;
  manufacturer: string;
  model: string;
  memory: number;
  bios_version: string;
  bios_date: string;
  uptime: number;
  support_end: string | null;
  gpus: Gpu[];
  disks: Disk[];
};
export type WinUpdateStatus = {
  updates: { title: string; kind: "Software" | "Driver"; size: number; mandatory: boolean }[];
  reboot_required: boolean;
  last_check: string;
};
export type AppUpdate = { name: string; id: string; version: string; available: string };
export const systemInfo = () => call<SystemInfo>("system_info");
export const windowsUpdates = () => call<WinUpdateStatus>("windows_updates");
export const openWindowsUpdate = () => call<void>("open_windows_update");
export const appUpdates = () => call<AppUpdate[]>("app_updates");
export const upgradeApps = (ids: string[]) => call<[number, string]>("upgrade_apps", { ids });

export type StartupItem = {
  id: string;
  name: string;
  command: string;
  source: "registry" | "folder" | "task" | "service";
  machine: boolean;
  enabled: boolean;
  can_toggle: boolean;
  keep: boolean;
  slow_ms: number;
  missing: boolean;
};
export type Boot = { time: number; boot_ms: number };
export const listStartup = () => call<{ items: StartupItem[]; boots: Boot[] }>("list_startup");
export const setStartup = (id: string, enabled: boolean) => call<void>("set_startup", { id, enabled });

export type BrokenShortcut = { path: string; target: string; place: "desktop" | "startmenu" | "taskbar"; modified: number; machine: boolean };
export const findShortcuts = () => call<BrokenShortcut[]>("find_shortcuts");
export const trashShortcuts = (paths: string[]) => call<ActionReport>("trash_shortcuts", { paths });

export type HealthItem = {
  id: string;
  group: "system" | "disk" | "security";
  status: "ok" | "warn" | "bad" | "info" | "unknown";
  title: Text;
  detail: Text;
  fix: string | null;
};
export const healthReport = () => call<{ items: HealthItem[]; elevated: boolean }>("health_report");
export const healthDism = () => call<HealthItem>("health_dism");
export const openSystemPage = (uri: string) => call<void>("open_system_page", { uri });

export type Ghost = { id: string; name: string; version: string; publisher: string; command: string };
export const findGhosts = () => call<Ghost[]>("find_ghosts");
export const removeGhosts = (ids: string[]) => call<number>("remove_ghosts", { ids });
export const restoreGhost = (file: string) => call<void>("restore_ghost", { file });
