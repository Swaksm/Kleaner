// Seul point de contact entre l'interface et le Rust.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

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
  duration_ms: number;
  top_files: Item[];
  top_dirs: Item[];
  categories: { category: Category; bytes: number; files: number }[];
};
export type ActionReport = { files: number; bytes: number };

export const readJournal = (limit = 200) => invoke<JournalView>("read_journal", { limit });
export const removeKleanerData = () => invoke<void>("remove_kleaner_data");
export const revealInExplorer = (path: string) => revealItemInDir(path);

export const startScan = () => invoke<void>("start_scan");
export const cancelScan = () => invoke<void>("cancel_scan");
export const onScanProgress = (fn: (p: ScanProgress) => void) => listen<ScanProgress>("scan-progress", (e) => fn(e.payload));
export const onScanDone = (fn: (r: ScanResult) => void) => listen<ScanResult>("scan-done", (e) => fn(e.payload));

export const moveToTrash = (paths: string[]) => invoke<ActionReport>("move_to_trash", { paths });
export const saveReport = (content: string) => invoke<string>("save_report", { content });

export type Text = { fr: string; en: string };
export type Cleaner = {
  id: string;
  group: "windows" | "browsers" | "apps" | "dev" | "system" | "heavy";
  label: Text;
  what: Text;
  why: Text;
  effect: Text;
  paths: string[];
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

export const listCleaners = () => invoke<CleanersView>("list_cleaners");
export const estimateCleaners = () => invoke<CleanReport[]>("estimate_cleaners");
export const runCleaners = (ids: string[]) => invoke<CleanReport[]>("run_cleaners", { ids });
export const relaunchAdmin = () => invoke<void>("relaunch_admin");
export const isElevated = () => invoke<boolean>("is_elevated");

export type Program = {
  id: string;
  name: string;
  publisher: string;
  version: string;
  bytes: number;
  install_date: string;
  location: string;
};
export const listPrograms = () => invoke<Program[]>("list_programs");
export const uninstallProgram = (id: string) => invoke<void>("uninstall_program", { id });
export const programSizes = () => invoke<{ id: string; bytes: number }[]>("program_sizes");
