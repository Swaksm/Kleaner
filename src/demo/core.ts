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
    case "run_cleaners":
      return ok(estimates().filter((r) => (args.ids as string[]).includes(r.id)));
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
    case "is_store":
      return ok(false);
    case "monthly_reminder":
      return ok(true);
    case "set_monthly_reminder":
      return ok(undefined);
    case "onedrive_roots":
    case "list_programs":
    case "find_duplicates":
      return ok([]);
    case "find_advice":
      return ok([
        { kind: "game", path: "D:\\Games\\SteamLibrary\\steamapps\\common\\Big Game", name: "Big Game", size: 38_400_000_000, modified: 1_745_000_000, app_id: 1 },
        { kind: "archive", path: "C:\\Users\\alex\\Downloads\\photos-vacances.zip", name: "photos-vacances.zip", size: 2_150_000_000, modified: 1_758_000_000, app_id: null },
        { kind: "installer", path: "C:\\Users\\alex\\Downloads\\VSCodeUserSetup-x64.exe", name: "VSCodeUserSetup-x64.exe", size: 98_000_000, modified: 1_752_000_000, app_id: null },
      ]);
    case "find_preinstalled":
      return ok([
        { id: "king.com.CandyCrushSaga_1.0_x86__kgqvnymyfvs32", name: "Candy Crush (King)", package: "king.com.CandyCrushSaga", family: "king.com.CandyCrushSaga_kgqvnymyfvs32", size: 210_000_000, warn: null },
        { id: "Microsoft.BingNews_4.0_x64__8wekyb3d8bbwe", name: "Microsoft News", package: "Microsoft.BingNews", family: "Microsoft.BingNews_8wekyb3d8bbwe", size: 30_000_000, warn: null },
        { id: "Microsoft.GamingApp_2.0_x64__8wekyb3d8bbwe", name: "Xbox", package: "Microsoft.GamingApp", family: "Microsoft.GamingApp_8wekyb3d8bbwe", size: 320_000_000, warn: "xbox" },
      ]);
    case "remove_preinstalled":
      return ok((args.ids as string[]).map((id) => ({ id, error: null })));
    case "open_store_page":
      return ok(undefined);
    case "freed_total":
      return ok([51_800_000_000, 1_790_000_000]);
    case "read_journal":
      return ok({ path: "C:\\Users\\alex\\AppData\\Local\\Kleaner\\journal.log", entries: [] });
    case "plan_move": {
      const source = args.path as string;
      const rel = source.replace(/^C:\\Users\\[^\\]+\\/, "");
      return ok({
        source,
        files: 1_284,
        bytes: 38_600_000_000,
        targets: [{ root: "D:\\", dest: `D:\\Kleaner moved\\${rel}`, free: 812_000_000_000 }],
      });
    }
    default:
      return Promise.reject(new Error(`demo : ${cmd} n'est pas simulé`));
  }
}
