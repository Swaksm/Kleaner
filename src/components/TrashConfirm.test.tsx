// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { setLang } from "../i18n/useT";
import { TrashConfirm } from "./TrashConfirm";

beforeAll(() => {
  // jsdom n'implémente pas <dialog>.showModal.
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
  setLang("fr");
});

afterEach(cleanup);

test("affiche nombre, taille totale, mode Corbeille et chaque chemin", () => {
  const items = [
    { path: "C:\\Users\\me\\Videos\\film.mkv", size: 2 * 1024 ** 3, modified: 0 },
    { path: "C:\\Users\\me\\Downloads\\setup.iso", size: 1024 ** 3, modified: 0 },
  ];
  render(<TrashConfirm items={items} onConfirm={vi.fn()} onCancel={vi.fn()} />);
  expect(screen.getByText("3,0 Go")).toBeTruthy();
  expect(screen.getByText("2")).toBeTruthy();
  expect(screen.getByText(/récupérable/)).toBeTruthy();
  for (const i of items) expect(screen.getByText(i.path)).toBeTruthy();
});

test("fermée quand rien n'est sélectionné", () => {
  const { container } = render(<TrashConfirm items={[]} onConfirm={vi.fn()} onCancel={vi.fn()} />);
  expect(container.querySelector("dialog")?.open).toBe(false);
});
