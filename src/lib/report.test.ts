import { expect, test } from "vitest";
import type { Cleaner, ScanResult } from "./api";
import { anonymize, buildReport } from "./report";

test("masque le nom d'utilisateur, pas les dossiers partagés", () => {
  expect(anonymize("C:\\Users\\marie\\Videos\\film.mkv")).toBe("%USERPROFILE%\\Videos\\film.mkv");
  expect(anonymize("C:\\Users\\marie")).toBe("%USERPROFILE%");
  expect(anonymize("C:\\Users\\Public\\x.iso")).toBe("C:\\Users\\Public\\x.iso");
  expect(anonymize("D:\\Jeux\\x.pak")).toBe("D:\\Jeux\\x.pak");
});

const scan: ScanResult = {
  roots: ["C:\\"],
  bytes: 10 * 1024 ** 3,
  files: 3,
  denied: 2,
  cancelled: false,
  full_access: false,
  fast: false,
  videos: [],
  images: [],
  drives: [],
  artifacts: [],
  tree: [],
  duration_ms: 1000,
  top_files: [{ path: "C:\\Users\\marie\\Videos\\film.mkv", size: 4 * 1024 ** 3, modified: 1_600_000_000 }],
  top_dirs: [{ path: "C:\\Users\\marie\\Projets\\vieux", size: 5 * 1024 ** 3, modified: 1_500_000_000 }],
  categories: [{ category: "videos", bytes: 4 * 1024 ** 3, files: 1 }],
};

const text = { fr: "x", en: "x" };
const cleaner = (id: string, admin: boolean): Cleaner => ({
  id,
  group: "windows",
  label: { fr: `Nettoyage ${id}`, en: id },
  what: text,
  why: text,
  effect: { fr: "Rien.", en: "Nothing." },
  paths: [],
  only: [],
  flat: false,
  admin,
  mode: "delete",
  level: "safe",
  close: [],
  command: null,
});

test("rapport : prompt, données anonymisées, nettoyages utiles seulement", () => {
  const md = buildReport({
    scan,
    cleaners: [cleaner("temp", false), cleaner("vide", false), cleaner("systeme", true)],
    estimates: [
      { id: "temp", files: 5, bytes: 2 * 1024 ** 3, skipped: 0, blocked_by: [], output: null, error: null },
      { id: "vide", files: 0, bytes: 0, skipped: 0, blocked_by: [], output: null, error: null },
    ],
    elevated: false,
    programs: [{ id: "HKCU\\x", name: "Jeu | Deluxe", publisher: "Studio", version: "1", bytes: 1024 ** 3, install_date: "20210101", location: "" }],
    lang: "fr",
    now: new Date("2026-10-13"),
  });
  expect(md).toContain("Pose-moi des questions courtes");
  expect(md).toContain("%USERPROFILE%\\Videos\\film.mkv");
  expect(md).not.toContain("marie");
  expect(md).toContain("| Nettoyage temp | 2,0 Go | Rien. |  |");
  expect(md).not.toContain("Nettoyage vide");
  expect(md).toContain("| Nettoyage systeme | inconnue | Rien. | oui |");
  expect(md).toContain("Jeu \\| Deluxe");
  expect(md).toContain("2020-09-13");
});
