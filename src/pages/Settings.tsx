import { useState } from "react";
import { Card } from "../components/Card";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { StatusPill } from "../components/StatusPill";
import { Switch } from "../components/Switch";
import { setLang, useT, type Lang } from "../i18n/useT";
import { removeKleanerData } from "../lib/api";
import { applyTheme, initialTheme } from "../lib/theme";

export function Settings() {
  const { t, lang } = useT();
  const [dark, setDark] = useState(initialTheme() === "dark");
  const [confirm, setConfirm] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function toggleTheme(next: boolean) {
    setDark(next);
    applyTheme(next ? "dark" : "light");
  }

  function remove() {
    setConfirm(false);
    removeKleanerData().catch((e) => setError(String(e)));
  }

  return (
    <>
      <Card title={t("settings.appearance")}>
        <Switch checked={dark} onChange={toggleTheme} label={t("settings.darkTheme")} />
        <label>
          {t("settings.language")}{" "}
          <select value={lang} onChange={(e) => setLang(e.target.value as Lang)}>
            <option value="en">English</option>
            <option value="fr">Français</option>
          </select>
        </label>
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
