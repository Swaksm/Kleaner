import { useEffect, useState } from "react";
import { StatusPill } from "./components/StatusPill";
import { Disclaimer, isDisclaimerAccepted } from "./components/Disclaimer";
import { useT, type Key } from "./i18n/useT";
import { isElevated } from "./lib/api";
import { Analyse } from "./pages/Analyse";
import { Cleanup } from "./pages/Cleanup";
import { Journal } from "./pages/Journal";
import { Programs } from "./pages/Programs";
import { Settings } from "./pages/Settings";

const pages = ["analyse", "cleanup", "programs", "journal", "settings"] as const;
type Page = (typeof pages)[number];

function App() {
  const { t } = useT();
  const [accepted, setAccepted] = useState(isDisclaimerAccepted());
  const [page, setPage] = useState<Page>("analyse");
  const [elevated, setElevated] = useState(false);

  useEffect(() => {
    isElevated().then(setElevated, () => {});
  }, []);

  if (!accepted) return <Disclaimer onAccept={() => setAccepted(true)} />;

  return (
    <div className="shell">
      <nav className="sidebar">
        <div className="brand">Kleaner</div>
        {elevated && (
          <div className="brand-badge">
            <StatusPill status="ok">{t("admin.badge")}</StatusPill>
          </div>
        )}
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
        {/* Analyse reste montée : le résultat du scan survit aux changements d'onglet. */}
        <div className="page" hidden={page !== "analyse"}>
          <Analyse />
        </div>
        {page === "journal" && <Journal />}
        {page === "settings" && <Settings />}
        {page === "cleanup" && <Cleanup />}
        {page === "programs" && <Programs />}
      </main>
    </div>
  );
}

export default App;
