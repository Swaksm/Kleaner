import { useSyncExternalStore } from "react";

// Action en cours (nettoyage, Corbeille, désinstallation…) : la fenêtre d'attente la montre et fige l'appli,
// pour qu'un changement de page ne l'interrompe pas.
export type Busy = {
  /** Étapes faites / total, connues seulement si Rust envoie `action-progress`. */
  done: number;
  total: number;
  label: string;
  /** Rend lisible l'étiquette reçue de Rust (identifiant d'un nettoyage). */
  name?: (label: string) => string;
  onStop?: () => void;
  stopping: boolean;
};

let current: Busy | null = null;
const listeners = new Set<() => void>();

function set(next: Busy | null) {
  current = next;
  listeners.forEach((fn) => fn());
}

export function setProgress(done: number, total: number, label: string) {
  if (current) set({ ...current, done, total, label });
}

export function requestStop() {
  if (!current?.onStop || current.stopping) return;
  current.onStop();
  set({ ...current, stopping: true });
}

/** Lance `fn` derrière la fenêtre d'attente. `onStop` : un bouton Arrêter l'appelle. */
export async function withBusy<T>(fn: () => Promise<T>, opts: { onStop?: () => void; name?: (label: string) => string } = {}): Promise<T> {
  set({ done: 0, total: 0, label: "", stopping: false, ...opts });
  try {
    return await fn();
  } finally {
    set(null);
  }
}

function subscribe(fn: () => void) {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

export function useBusy() {
  return useSyncExternalStore(subscribe, () => current);
}
