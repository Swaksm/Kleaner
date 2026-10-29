import { useState } from "react";
import { LANGS, setLang, useT } from "../i18n/useT";
import { chooseAutostart } from "../lib/autostart";
import { applyThemeChoice, savedThemeChoice, type ThemeChoice } from "../lib/theme";
import { useIsStore } from "../lib/store";
import { setUpdateCheck, updateCheckEnabled } from "../lib/update";
import { Flag } from "./LangMenu";
import { requestTour } from "./Tour";

const STORAGE_KEY = "setupDone";

export function isSetupDone(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

/** Assistant du premier lancement : langue, thème, mises à jour, démarrage avec Windows. Tout reste modifiable dans les Paramètres. */
export function Setup({ elevated, onDone }: { elevated: boolean; onDone: () => void }) {
  const { t, lang } = useT();
  const [theme, setTheme] = useState<ThemeChoice>(savedThemeChoice());
  const [update, setUpdate] = useState(updateCheckEnabled());
  const [autostart, setAutostartChoice] = useState(false);
  const store = useIsStore();

  function finish() {
    setUpdateCheck(update);
    try {
      localStorage.setItem(STORAGE_KEY, "1");
    } catch {}
    requestTour();
    // Démarrage auto : seul choix qui demande les droits admin (relance de Kleaner si besoin).
    if (autostart) chooseAutostart(true, elevated).catch(() => {});
    onDone();
  }

  return (
    <div className="disclaimer">
      <div className="card disclaimer-card setup">
        <span className="brand-logo brand-logo-big" aria-hidden />
        <h1>{t("setup.title")}</h1>
        <p className="muted">{t("setup.intro")}</p>

        <fieldset className="setup-langs">
          <legend>{t("settings.language")}</legend>
          {LANGS.map(([l, name]) => (
            <button key={l} className={`btn ${l === lang ? "btn-selected" : ""}`} aria-pressed={l === lang} onClick={() => setLang(l)}>
              <Flag lang={l} />
              {name}
            </button>
          ))}
        </fieldset>

        <label>
          {t("settings.theme")}{" "}
          <select
            value={theme}
            onChange={(e) => {
              const next = e.target.value as ThemeChoice;
              setTheme(next);
              applyThemeChoice(next);
            }}
          >
            <option value="system">{t("settings.themeSystem")}</option>
            <option value="light">{t("settings.themeLight")}</option>
            <option value="dark">{t("settings.themeDark")}</option>
          </select>
        </label>

        {store === false && (
          <>
            <label className="check">
              <input type="checkbox" checked={update} onChange={(e) => setUpdate(e.target.checked)} />
              {t("update.auto")}
            </label>
            <p className="muted small">{t("update.privacy")}</p>
          </>
        )}

        <label className="check">
          <input type="checkbox" checked={autostart} onChange={(e) => setAutostartChoice(e.target.checked)} />
          {t(store ? "autostart.labelStore" : "autostart.label")}
        </label>
        <p className="muted small">{t(store ? "autostart.noteStore" : "autostart.note")}</p>

        <button className="btn btn-primary" onClick={finish}>
          {t("setup.finish")}
        </button>
      </div>
    </div>
  );
}
