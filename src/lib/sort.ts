export type SortDir = "asc" | "desc";
export type SortState<K extends string> = { key: K; dir: SortDir };

/** Nom de fichier (ou dernier segment) d'un chemin Windows. */
export const baseName = (path: string) => path.slice(path.lastIndexOf("\\") + 1);

/** Trie une copie de `items`. Les textes se comparent sans tenir compte de la casse ni des accents. */
export function sortBy<T, K extends string>(
  items: T[],
  state: SortState<K>,
  getters: Record<K, (item: T) => string | number>,
): T[] {
  const get = getters[state.key];
  const sign = state.dir === "asc" ? 1 : -1;
  const collator = new Intl.Collator(undefined, { sensitivity: "base", numeric: true });
  return [...items].sort((a, b) => {
    const x = get(a);
    const y = get(b);
    const c = typeof x === "number" && typeof y === "number" ? x - y : collator.compare(String(x), String(y));
    return c * sign;
  });
}

/** Clic sur la même clé : inverse le sens. Autre clé : texte croissant, nombres décroissants. */
export function nextSort<K extends string>(state: SortState<K>, key: K, numeric: boolean): SortState<K> {
  if (state.key === key) return { key, dir: state.dir === "asc" ? "desc" : "asc" };
  return { key, dir: numeric ? "desc" : "asc" };
}
