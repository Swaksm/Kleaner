import { useSyncExternalStore } from "react";
import fr from "./fr.json";
import en from "./en.json";

export type Lang = "fr" | "en";
export type Key = keyof typeof fr;

// Le typage force EN à contenir toutes les clés FR.
const dicts: Record<Lang, Record<Key, string>> = { fr, en };

function initialLang(): Lang {
  try {
    const saved = localStorage.getItem("lang");
    if (saved === "fr" || saved === "en") return saved;
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

export function useT() {
  const current = useSyncExternalStore(subscribe, () => lang);
  return { lang: current, t: (key: Key) => dicts[current][key] };
}
