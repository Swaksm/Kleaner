import { expect, test } from "vitest";
import { childrenOf, parentOf, squarify } from "./treemap";

test("parents et enfants, racine « C:\\ » comprise", () => {
  expect(parentOf("C:\\Users\\me")).toBe("C:\\Users");
  expect(parentOf("C:\\Users")).toBe("C:");
  const tree = [
    { path: "C:\\", size: 100, modified: 0 },
    { path: "C:\\Users", size: 60, modified: 0 },
    { path: "C:\\Windows", size: 30, modified: 0 },
    { path: "C:\\Users\\me", size: 50, modified: 0 },
  ];
  expect(childrenOf(tree, "C:\\").map((i) => i.path)).toEqual(["C:\\Users", "C:\\Windows"]);
});

test("les aires sont proportionnelles et couvrent la boîte", () => {
  const tiles = squarify([6, 6, 4, 3, 2, 2, 1], (v) => v, { x: 0, y: 0, w: 6, h: 4 });
  expect(tiles).toHaveLength(7);
  const area = tiles.reduce((s, t) => s + t.w * t.h, 0);
  expect(area).toBeCloseTo(24, 6);
  for (const t of tiles) expect(t.w * t.h).toBeCloseTo(t.item, 6);
  for (const t of tiles) {
    expect(t.x + t.w).toBeLessThanOrEqual(6 + 1e-9);
    expect(t.y + t.h).toBeLessThanOrEqual(4 + 1e-9);
  }
});
