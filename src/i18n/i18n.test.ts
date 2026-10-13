import { expect, test } from "vitest";
import fr from "./fr.json";
import en from "./en.json";

test("FR et EN ont exactement les mêmes clés", () => {
  expect(Object.keys(en).sort()).toEqual(Object.keys(fr).sort());
});

test("aucune traduction vide", () => {
  for (const dict of [fr, en]) {
    for (const value of Object.values(dict)) expect(value.trim()).not.toBe("");
  }
});
