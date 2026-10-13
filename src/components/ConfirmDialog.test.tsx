// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { ConfirmDialog } from "./ConfirmDialog";

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
});

afterEach(cleanup);

function dialog(confirmDisabled: boolean, onConfirm = vi.fn()) {
  render(
    <ConfirmDialog
      open
      title="Hibernation"
      confirmLabel="Lancer"
      cancelLabel="Annuler"
      danger
      confirmDisabled={confirmDisabled}
      onConfirm={onConfirm}
      onCancel={vi.fn()}
    >
      <p>conséquence</p>
    </ConfirmDialog>,
  );
  return onConfirm;
}

test("nettoyage lourd : bouton bloqué tant que la conséquence n'est pas acceptée", () => {
  const onConfirm = dialog(true);
  const btn = screen.getByText("Lancer") as HTMLButtonElement;
  expect(btn.disabled).toBe(true);
  fireEvent.click(btn);
  expect(onConfirm).not.toHaveBeenCalled();
});

test("bouton actif une fois accepté", () => {
  const onConfirm = dialog(false);
  fireEvent.click(screen.getByText("Lancer"));
  expect(onConfirm).toHaveBeenCalledOnce();
});
