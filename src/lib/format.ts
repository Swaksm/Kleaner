import type { Lang } from "../i18n/useT";

const units: Record<Lang, string[]> = {
  fr: ["o", "Ko", "Mo", "Go", "To"],
  en: ["B", "KB", "MB", "GB", "TB"],
};

/** Taille en unités binaires (1 Ko = 1024 o), 1 décimale au-delà de l'octet. */
export function formatBytes(bytes: number, lang: Lang): string {
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < units[lang].length - 1) {
    v /= 1024;
    i++;
  }
  const n = i === 0 ? String(v) : v.toLocaleString(lang, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
  return `${n} ${units[lang][i]}`;
}

export function formatDate(ts: number, lang: Lang): string {
  return new Date(ts * 1000).toLocaleString(lang, { dateStyle: "medium", timeStyle: "short" });
}
