import { useSyncExternalStore } from "react";
import de from "./de.json";
import en from "./en.json";
import es from "./es.json";
import fr from "./fr.json";
import it from "./it.json";
import pt from "./pt.json";
import ru from "./ru.json";
import zh from "./zh.json";

export type Lang = "en" | "fr" | "es" | "de" | "it" | "pt" | "ru" | "zh";
export const LANGS: [Lang, string][] = [
  ["en", "English"],
  ["fr", "Français"],
  ["es", "Español"],
  ["de", "Deutsch"],
  ["it", "Italiano"],
  ["pt", "Português"],
  ["ru", "Русский"],
  ["zh", "中文"],
];
export type TextLang = "fr" | "en";
export type Key = keyof typeof en;

// Le typage force chaque langue à contenir toutes les clés de l'anglais.
const dicts: Record<Lang, Record<Key, string>> = { en, fr, es, de, it, pt, ru, zh };

function initialLang(): Lang {
  try {
    const saved = localStorage.getItem("lang");
    if (LANGS.some(([l]) => l === saved)) return saved as Lang;
  } catch {}
  // English first : le français se choisit dans les Paramètres.
  return "en";
}

let lang = initialLang();
const listeners = new Set<() => void>();

export function setLang(next: Lang) {
  lang = next;
  document.documentElement.lang = next;
  try {
    localStorage.setItem("lang", next);
  } catch {}
  listeners.forEach((fn) => fn());
}

function subscribe(fn: () => void) {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

/** Texte dans la langue courante, hors composant React (messages d'erreur). */
export function translate(key: Key): string {
  return dicts[lang][key];
}

export function useT() {
  const current = useSyncExternalStore(subscribe, () => lang);
  // Textes du catalogue et des fiches : anglais et français seulement, anglais par défaut.
  const tl: TextLang = current === "fr" ? "fr" : "en";
  return { lang: current, tl, t: (key: Key) => dicts[current][key] };
}
