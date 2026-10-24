// Remplace @tauri-apps/api/core dans le mode « demo » : des réponses fictives, rien ne touche au disque.
import { cleaners, drives, estimates, explain, health, leftovers, scanResult, startupView } from "./data";
import { emit } from "./event";

export const convertFileSrc = (path: string) => path;

export function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const ok = (v: unknown) => Promise.resolve(v as T);
  switch (cmd) {
    case "list_drives":
      return ok(drives());
    case "start_scan":
      setTimeout(() => emit("scan-progress", { files: 640_000, bytes: 380 * 1024 ** 3, denied: 0 }), 200);
      setTimeout(() => emit("scan-done", scanResult()), 700);
      return ok(undefined);
    case "explain_paths":
      return ok((args.paths as string[]).map(explain));
    case "list_cleaners":
      return ok({ elevated: false, cleaners: cleaners() });
    case "estimate_cleaners":
      return ok(estimates());
    case "find_leftovers":
      return ok(leftovers());
    case "list_startup":
      return ok(startupView());
    case "health_report":
      return ok({ items: health(), elevated: false });
    case "is_elevated":
    case "autostart_enabled":
    case "mica":
    case "is_installed":
      return ok(false);
    case "onedrive_roots":
    case "list_programs":
    case "find_duplicates":
      return ok([]);
    case "freed_total":
      return ok([51_800_000_000, 1_790_000_000]);
    case "read_journal":
      return ok({ path: "C:\\Users\\alex\\AppData\\Local\\Kleaner\\journal.log", entries: [] });
    default:
      return Promise.reject(new Error(`demo : ${cmd} n'est pas simulé`));
  }
}
