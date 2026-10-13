import { useState } from "react";
import { Card } from "./components/Card";
import { StatusPill } from "./components/StatusPill";
import { Switch } from "./components/Switch";
import { setLang, useT, type Key, type Lang } from "./i18n/useT";
import { applyTheme, initialTheme } from "./lib/theme";

const pages = ["analyse", "cleanup", "programs", "journal", "settings"] as const;
type Page = (typeof pages)[number];

function App() {
  const { t, lang } = useT();
  const [page, setPage] = useState<Page>("analyse");
  const [dark, setDark] = useState(initialTheme() === "dark");

  function toggleTheme(next: boolean) {
    setDark(next);
    applyTheme(next ? "dark" : "light");
  }

  return (
    <div className="shell">
      <nav className="sidebar">
        <div className="brand">Kleaner</div>
        {pages.map((p) => (
          <button
            key={p}
            className="nav-item"
            aria-current={page === p ? "page" : undefined}
            onClick={() => setPage(p)}
          >
            {t(`nav.${p}` as Key)}
          </button>
        ))}
        <div className="sidebar-footer">
          <Switch checked={dark} onChange={toggleTheme} label={t("settings.darkTheme")} />
          <label>
            {t("settings.language")}{" "}
            <select value={lang} onChange={(e) => setLang(e.target.value as Lang)}>
              <option value="fr">Français</option>
              <option value="en">English</option>
            </select>
          </label>
        </div>
      </nav>
      <main className="content">
        <h1>{t(`nav.${page}` as Key)}</h1>
        <Card>
          <StatusPill status="neutral">{t("status.planned")}</StatusPill>
          <p>{t("page.soon")}</p>
        </Card>
      </main>
    </div>
  );
}

export default App;
