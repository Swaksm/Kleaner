import { useState } from "react";
import { Card } from "./components/Card";
import { Disclaimer, isDisclaimerAccepted } from "./components/Disclaimer";
import { StatusPill } from "./components/StatusPill";
import { useT, type Key } from "./i18n/useT";
import { Journal } from "./pages/Journal";
import { Settings } from "./pages/Settings";

const pages = ["analyse", "cleanup", "programs", "journal", "settings"] as const;
type Page = (typeof pages)[number];

function Placeholder() {
  const { t } = useT();
  return (
    <Card>
      <StatusPill status="neutral">{t("status.planned")}</StatusPill>
      <p>{t("page.soon")}</p>
    </Card>
  );
}

function App() {
  const { t } = useT();
  const [accepted, setAccepted] = useState(isDisclaimerAccepted());
  const [page, setPage] = useState<Page>("analyse");

  if (!accepted) return <Disclaimer onAccept={() => setAccepted(true)} />;

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
      </nav>
      <main className="content">
        <h1>{t(`nav.${page}` as Key)}</h1>
        {page === "journal" ? <Journal /> : page === "settings" ? <Settings /> : <Placeholder />}
      </main>
    </div>
  );
}

export default App;
