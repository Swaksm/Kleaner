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

// Incrémenté à la fin de chaque action qui modifie le disque : l'espace libre affiché se relit tout seul.
let diskVersion = 0;
const diskListeners = new Set<() => void>();

/** Lance `fn` derrière la fenêtre d'attente. `onStop` : un bouton Arrêter l'appelle. */
export async function withBusy<T>(fn: () => Promise<T>, opts: { onStop?: () => void; name?: (label: string) => string } = {}): Promise<T> {
  set({ done: 0, total: 0, label: "", stopping: false, ...opts });
  try {
    return await fn();
  } finally {
    set(null);
    diskVersion++;
    diskListeners.forEach((fn) => fn());
  }
}

/** Change après chaque action sur le disque : à mettre dans les dépendances d'un effet qui relit l'espace libre. */
export function useDiskVersion() {
  return useSyncExternalStore(
    (fn) => {
      diskListeners.add(fn);
      return () => {
        diskListeners.delete(fn);
      };
    },
    () => diskVersion,
  );
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
