export type Theme = "light" | "dark";
// Clé « themeChoice » (et non plus « theme ») : l'ancien interrupteur enregistrait un choix figé.
export type ThemeChoice = Theme | "system";

const media = () => matchMedia("(prefers-color-scheme: dark)");

/** Choix enregistré ; « system » par défaut (suit Windows). */
export function savedThemeChoice(): ThemeChoice {
  try {
    const saved = localStorage.getItem("themeChoice");
    if (saved === "light" || saved === "dark" || saved === "system") return saved;
  } catch {}
  return "system";
}

export function resolveTheme(choice: ThemeChoice): Theme {
  if (choice !== "system") return choice;
  return media().matches ? "dark" : "light";
}

let stopFollowing: (() => void) | null = null;

/** Applique le choix ; en mode « system », suit les changements de Windows en direct. */
export function applyThemeChoice(choice: ThemeChoice, remember = true) {
  document.documentElement.dataset.theme = resolveTheme(choice);
  stopFollowing?.();
  stopFollowing = null;
  if (choice === "system") {
    const m = media();
    const onChange = () => (document.documentElement.dataset.theme = resolveTheme("system"));
    m.addEventListener("change", onChange);
    stopFollowing = () => m.removeEventListener("change", onChange);
  }
  if (!remember) return;
  try {
    localStorage.setItem("themeChoice", choice);
  } catch {}
}
