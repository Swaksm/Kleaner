import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { StatusPill } from "../components/StatusPill";
import { LANGS, setLang, useT, type Lang } from "../i18n/useT";
import { isElevated, removeKleanerData } from "../lib/api";
import { autostartEnabled, chooseAutostart } from "../lib/autostart";
import { applyThemeChoice, savedThemeChoice, type ThemeChoice } from "../lib/theme";
import { setUpdateCheck, updateCheckEnabled } from "../lib/update";

export function Settings({ onTour }: { onTour: () => void }) {
  const { t, lang } = useT();
  const [theme, setTheme] = useState<ThemeChoice>(savedThemeChoice());
  const [confirm, setConfirm] = useState(false);
  const [autoUpdate, setAutoUpdate] = useState(updateCheckEnabled());
  const [error, setError] = useState<string | null>(null);
  const [autostart, setAutostartState] = useState<boolean | null>(null);
  const [elevated, setElevated] = useState(false);

  useEffect(() => {
    autostartEnabled().then(setAutostartState, () => setAutostartState(false));
    isElevated().then(setElevated, () => {});
  }, []);

  function changeAutostart(on: boolean) {
    chooseAutostart(on, elevated).then(
      () => setAutostartState(on),
      (e) => setError(String(e)),
    );
  }

  function changeTheme(next: ThemeChoice) {
    setTheme(next);
    applyThemeChoice(next);
  }

  function remove() {
    setConfirm(false);
    removeKleanerData().catch((e) => setError(String(e)));
  }

  return (
    <>
      <Card title={t("settings.appearance")}>
        <label>
          {t("settings.theme")}{" "}
          <select value={theme} onChange={(e) => changeTheme(e.target.value as ThemeChoice)}>
            <option value="system">{t("settings.themeSystem")}</option>
            <option value="light">{t("settings.themeLight")}</option>
            <option value="dark">{t("settings.themeDark")}</option>
          </select>
        </label>
        <label>
          {t("settings.language")}{" "}
          <select value={lang} onChange={(e) => setLang(e.target.value as Lang)}>
            {LANGS.map(([code, name]) => (
              <option key={code} value={code}>
                {name}
              </option>
            ))}
          </select>
        </label>
        <div className="row">
          <button className="btn" onClick={onTour}>
            {t("tour.replay")}
          </button>
        </div>
      </Card>

      <Card title={t("update.title")}>
        <label className="check">
          <input
            type="checkbox"
            checked={autoUpdate}
            onChange={(e) => {
              setAutoUpdate(e.target.checked);
              setUpdateCheck(e.target.checked);
            }}
          />
          {t("update.auto")}
        </label>
        <p className="muted small">{t("update.privacy")}</p>
      </Card>

      <Card title={t("autostart.title")}>
        <label className="check">
          <input type="checkbox" disabled={autostart === null} checked={!!autostart} onChange={(e) => changeAutostart(e.target.checked)} />
          {t("autostart.label")}
        </label>
        <p className="muted small">{t("autostart.note")}</p>
      </Card>

      <Card title={t("settings.remove.title")}>
        <p>{t("settings.remove.what")}</p>
        <p className="muted">{t("settings.remove.exe")}</p>
        <button className="btn btn-danger" onClick={() => setConfirm(true)}>
          {t("settings.remove.button")}
        </button>
        {error && <StatusPill status="danger">{error}</StatusPill>}
      </Card>

      <ConfirmDialog
        open={confirm}
        title={t("settings.remove.confirmTitle")}
        confirmLabel={t("settings.remove.button")}
        cancelLabel={t("common.cancel")}
        danger
        onConfirm={remove}
        onCancel={() => setConfirm(false)}
      >
        <p>{t("settings.remove.what")}</p>
        <p>{t("settings.remove.irreversible")}</p>
      </ConfirmDialog>
    </>
  );
}
