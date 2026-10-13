import { expect, test } from "vitest";
import { baseName, nextSort, sortBy } from "./sort";

const items = [
  { path: "C:\\b\\Été.mp4", size: 10, modified: 3 },
  { path: "C:\\a\\avion.mp4", size: 30, modified: 1 },
  { path: "C:\\c\\Zèbre.mp4", size: 20, modified: 2 },
];
const getters = { name: (i: (typeof items)[0]) => baseName(i.path), size: (i: (typeof items)[0]) => i.size };

test("tri par taille décroissante et par nom sans casse ni accents", () => {
  expect(sortBy(items, { key: "size", dir: "desc" }, getters).map((i) => i.size)).toEqual([30, 20, 10]);
  expect(sortBy(items, { key: "name", dir: "asc" }, getters).map((i) => baseName(i.path))).toEqual([
    "avion.mp4",
    "Été.mp4",
    "Zèbre.mp4",
  ]);
});

test("cliquer deux fois inverse le sens ; nouvelle clé numérique commence décroissante", () => {
  expect(nextSort({ key: "name", dir: "asc" }, "name", false)).toEqual({ key: "name", dir: "desc" });
  expect(nextSort({ key: "name", dir: "asc" }, "size", true)).toEqual({ key: "size", dir: "desc" });
});

test("baseName", () => {
  expect(baseName("C:\\x\\y\\photo.jpg")).toBe("photo.jpg");
});
