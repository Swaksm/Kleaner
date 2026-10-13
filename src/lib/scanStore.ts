import { useSyncExternalStore } from "react";
import type { ScanResult } from "./api";

// Dernier résultat d'analyse, partagé entre les onglets Analyse et Photos & vidéos.
let current: ScanResult | null = null;
const listeners = new Set<() => void>();

export function setScanResult(next: ScanResult | null) {
  current = next;
  listeners.forEach((fn) => fn());
}

/** Retire des chemins envoyés à la Corbeille de toutes les listes du résultat. */
export function forgetPaths(paths: string[]) {
  if (!current) return;
  const gone = new Set(paths);
  const keep = <T extends { path: string }>(l: T[]) => l.filter((i) => !gone.has(i.path));
  setScanResult({
    ...current,
    top_files: keep(current.top_files),
    videos: keep(current.videos),
    images: keep(current.images),
    artifacts: keep(current.artifacts),
  });
}

function subscribe(fn: () => void) {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

export function useScanResult() {
  return useSyncExternalStore(subscribe, () => current);
}
