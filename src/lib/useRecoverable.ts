import { useEffect, useState, useSyncExternalStore } from "react";
import { estimateCleaners, listCleaners, type ScanResult } from "./api";

// Un seul calcul par analyse, partagé entre l'Accueil et l'Analyse (l'estimation relit les caches).
let cache: { key: number; value: Promise<number> } | null = null;

// Incrémenté après un nettoyage : l'estimation est refaite partout où elle est affichée.
let version = 0;
const listeners = new Set<() => void>();

export function forgetRecoverable() {
  cache = null;
  version++;
  listeners.forEach((fn) => fn());
}

function subscribe(fn: () => void) {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

function compute(): Promise<number> {
  return Promise.all([listCleaners(), estimateCleaners()]).then(([view, list]) => {
    const byId = new Map(view.cleaners.map((c) => [c.id, c]));
    return list.filter((r) => byId.get(r.id)?.level === "safe" && byId.get(r.id)?.group !== "community" && r.blocked_by.length === 0).reduce((n, r) => n + r.bytes, 0);
  });
}

/**
 * Espace libérable sans risque par le Nettoyage (mêmes règles que « Cocher les éléments sans risque »),
 * recalculé après chaque analyse et chaque nettoyage. `null` pendant le calcul.
 */
export function useRecoverable(scan: ScanResult | null): number | null {
  const [recoverable, setRecoverable] = useState<number | null>(null);
  const key = scan?.duration_ms;
  const v = useSyncExternalStore(subscribe, () => version);
  useEffect(() => {
    if (key === undefined) return;
    setRecoverable(null);
    if (cache?.key !== key) cache = { key, value: compute() };
    let alive = true;
    cache.value.then((v) => alive && setRecoverable(v), () => {});
    return () => {
      alive = false;
    };
  }, [key, v]);
  return recoverable;
}
