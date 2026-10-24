import { expect, test } from "vitest";
import { place } from "./Tour";

const W = 340;
const H = 200;

test("la bulle va à droite de la cible, sinon dessous, et reste dans la fenêtre", () => {
  // Élément de la barre latérale : à droite, centrée en hauteur.
  expect(place({ top: 150, left: 6, width: 207, height: 49 }, W, H, 1280, 720)).toEqual({ left: 227, top: 74.5 });
  // Pas de place à droite : dessous.
  expect(place({ top: 100, left: 900, width: 300, height: 40 }, W, H, 1280, 720)).toEqual({ left: 880, top: 154 });
  // Sans cible : au centre.
  expect(place(null, W, H, 1280, 720)).toEqual({ left: 470, top: 260 });
  // Cible en bas d'une petite fenêtre : la bulle ne sort jamais.
  const p = place({ top: 400, left: 6, width: 207, height: 57 }, W, H, 800, 464);
  expect(p.top + H).toBeLessThanOrEqual(464 - 16);
});
