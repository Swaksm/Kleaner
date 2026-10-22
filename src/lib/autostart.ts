// Démarrage avec Windows (tâche planifiée, réduit, admin). Changer le réglage demande les droits admin :
// sans eux, le choix est mémorisé, Kleaner se relance en admin et l'applique au démarrage suivant.
import { autostartEnabled, relaunchAdmin, setAutostart } from "./api";

const PENDING = "autostartPending";

export { autostartEnabled };

export async function chooseAutostart(on: boolean, elevated: boolean): Promise<void> {
  if (elevated) return setAutostart(on);
  try {
    localStorage.setItem(PENDING, on ? "1" : "0");
  } catch {}
  return relaunchAdmin();
}

/** À appeler une fois en admin : applique le choix fait avant la relance. */
export async function applyPendingAutostart(): Promise<void> {
  let pending: string | null = null;
  try {
    pending = localStorage.getItem(PENDING);
    localStorage.removeItem(PENDING);
  } catch {}
  if (pending === "1" || pending === "0") await setAutostart(pending === "1");
}
