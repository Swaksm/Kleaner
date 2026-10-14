import { useEffect, useState } from "react";
import { estimateCleaners, listCleaners, type ScanResult } from "./api";

// Un seul calcul par analyse, partagé entre l'Accueil et l'Analyse (l'estimation relit les caches).
let cache: { key: number; value: Promise<number> } | null = null;

function compute(): Promise<number> {
  return Promise.all([listCleaners(), estimateCleaners()]).then(([view, list]) => {
    const byId = new Map(view.cleaners.map((c) => [c.id, c]));
    return list.filter((r) => byId.get(r.id)?.level === "safe" && r.blocked_by.length === 0).reduce((n, r) => n + r.bytes, 0);
  });
}

/**
 * Espace libérable sans risque par le Nettoyage (mêmes règles que « Cocher les éléments sans risque »),
 * recalculé après chaque analyse. `null` pendant le calcul.
 */
export function useRecoverable(scan: ScanResult | null): number | null {
  const [recoverable, setRecoverable] = useState<number | null>(null);
  const key = scan?.duration_ms;
  useEffect(() => {
    if (key === undefined) return;
    setRecoverable(null);
    if (cache?.key !== key) cache = { key, value: compute() };
    let alive = true;
    cache.value.then((v) => alive && setRecoverable(v), () => {});
    return () => {
      alive = false;
    };
  }, [key]);
  return recoverable;
}
