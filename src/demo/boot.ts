// Préparation du mode « demo » : avertissement accepté, langue et thème de l'adresse (?lang=fr&theme=dark),
// et une analyse déjà faite (sauf ?scan=0).
import { setLang, type Lang } from "../i18n/useT";
import { applyThemeChoice, type ThemeChoice } from "../lib/theme";
import { setScanResult } from "../lib/scanStore";
import { scanResult } from "./data";

export function boot() {
  const q = new URLSearchParams(location.search);
  try {
    localStorage.setItem("disclaimerAccepted", "1");
  } catch {}
  setLang((q.get("lang") as Lang | null) ?? "en");
  applyThemeChoice((q.get("theme") as ThemeChoice | null) ?? "dark", false);
  if (q.get("scan") !== "0") setScanResult(scanResult());
  // ?scroll=<sélecteur CSS> : amène la carte voulue en haut de la fenêtre (capture d'écran de la liste des verdicts).
  const target = q.get("scroll");
  if (target) setTimeout(() => (document.querySelector(target)?.closest("section") ?? document.querySelector(target))?.scrollIntoView(), 3000);
}
