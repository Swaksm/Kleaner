// Version Microsoft Store : le Store gère les mises à jour, le démarrage avec Windows passe par le paquet (sans admin).
import { useEffect, useState } from "react";
import { isStore } from "./api";

/** `null` tant que la réponse n'est pas arrivée : les écrans masquent la mise à jour jusque-là. */
export function useIsStore(): boolean | null {
  const [store, setStore] = useState<boolean | null>(null);
  useEffect(() => {
    isStore().then(setStore, () => setStore(false));
  }, []);
  return store;
}
