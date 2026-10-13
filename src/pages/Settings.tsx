import { useState } from "react";
import { Card } from "../components/Card";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { StatusPill } from "../components/StatusPill";
import { setLang, useT, type Lang } from "../i18n/useT";
import { removeKleanerData } from "../lib/api";
import { applyThemeChoice, savedThemeChoice, type ThemeChoice } from "../lib/theme";

export function Settings() {
  const { t, lang } = useT();
  const [theme, setTheme] = useState<ThemeChoice>(savedThemeChoice());
  const [confirm, setConfirm] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
