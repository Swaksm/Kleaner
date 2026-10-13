// Seul point de contact entre l'interface et le Rust.
import { invoke } from "@tauri-apps/api/core";
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

export const readJournal = (limit = 200) => invoke<JournalView>("read_journal", { limit });
export const removeKleanerData = () => invoke<void>("remove_kleaner_data");
export const revealInExplorer = (path: string) => revealItemInDir(path);
