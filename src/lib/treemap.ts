// Carte du disque : arbre des gros dossiers et disposition « squarified » des rectangles.
import type { Item } from "./api";

export type Rect = { x: number; y: number; w: number; h: number };
export type Tile<T> = Rect & { item: T };

/** Chemin sans « \ » final : « C:\ » devient « C: ». */
export const norm = (p: string) => p.replace(/\\+$/, "");
export const parentOf = (p: string) => {
  const n = norm(p);
  const i = n.lastIndexOf("\\");
  return i < 0 ? "" : n.slice(0, i);
};

/** Enfants directs de `dir` présents dans l'arbre de l'analyse. */
export function childrenOf(tree: Item[], dir: string): Item[] {
  const d = norm(dir);
  return tree.filter((i) => parentOf(i.path) === d).sort((a, b) => b.size - a.size);
}

function worst(row: number[], side: number): number {
  const sum = row.reduce((a, b) => a + b, 0);
  const max = Math.max(...row);
  const min = Math.min(...row);
  return Math.max((side * side * max) / (sum * sum), (sum * sum) / (side * side * min));
}

/** Rectangles proportionnels aux valeurs, aussi carrés que possible (Bruls et al.). */
export function squarify<T>(items: T[], value: (t: T) => number, box: Rect): Tile<T>[] {
  const total = items.reduce((s, t) => s + value(t), 0);
  if (total <= 0 || box.w <= 0 || box.h <= 0) return [];
  const scale = (box.w * box.h) / total;
  const queue = items.filter((t) => value(t) > 0).map((t) => ({ t, a: value(t) * scale }));
  const out: Tile<T>[] = [];
  let { x, y, w, h } = box;
  while (queue.length > 0) {
    const side = Math.min(w, h);
    const row = [queue.shift()!];
    while (queue.length > 0 && worst([...row.map((r) => r.a), queue[0].a], side) <= worst(row.map((r) => r.a), side)) {
      row.push(queue.shift()!);
    }
    const sum = row.reduce((s, r) => s + r.a, 0);
    const thick = sum / side;
    let offset = 0;
    for (const r of row) {
      const len = r.a / thick;
      out.push(w >= h ? { x, y: y + offset, w: thick, h: len, item: r.t } : { x: x + offset, y, w: len, h: thick, item: r.t });
      offset += len;
    }
    if (w >= h) {
      x += thick;
      w -= thick;
    } else {
      y += thick;
      h -= thick;
    }
  }
  return out;
}
