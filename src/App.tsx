import { useEffect, useState } from "react";
import { StatusPill } from "./components/StatusPill";
import { Disclaimer, isDisclaimerAccepted } from "./components/Disclaimer";
import { UpdateBanner } from "./components/UpdateBanner";
import { useT, type Key } from "./i18n/useT";
import { isElevated, onedriveRoots } from "./lib/api";
import { useScanResult } from "./lib/scanStore";
import { About } from "./pages/About";
import { Analyse } from "./pages/Analyse";
import { Cleanup } from "./pages/Cleanup";
import { Duplicates } from "./pages/Duplicates";
import { Home } from "./pages/Home";
import { OneDrive } from "./pages/OneDrive";
import { Journal } from "./pages/Journal";
import { Media } from "./pages/Media";
import { Programs } from "./pages/Programs";
import { Projects } from "./pages/Projects";
import { Settings } from "./pages/Settings";

// L'essentiel, toujours visible ; outils avancés repliables ; pages secondaires en bas.
const MAIN = ["home", "analyse", "cleanup", "media", "programs"] as const;
const MORE = ["duplicates", "projects", "onedrive", "journal"] as const;
type Page = (typeof MAIN)[number] | (typeof MORE)[number] | "settings" | "about";

function savedMoreOpen(): boolean {
  try {
    return localStorage.getItem("navMore") === "1";
  } catch {
    return false;
  }
}

function App() {
  const { t } = useT();
  const [accepted, setAccepted] = useState(isDisclaimerAccepted());
  const [page, setPage] = useState<Page>("home");
  // Incrémenté par l'Accueil : l'Analyse lance alors une analyse.
  const [scanRequest, setScanRequest] = useState(0);
  const [hasOneDrive, setHasOneDrive] = useState(false);
  const [elevated, setElevated] = useState(false);
  const [moreOpen, setMoreOpen] = useState(savedMoreOpen);
  const scan = useScanResult();
  // « Vieux projets » n'apparaît que si l'analyse a trouvé des projets de code.
  const more = MORE.filter(
    (p) => (p !== "projects" || (scan?.artifacts.length ?? 0) > 0) && (p !== "onedrive" || hasOneDrive),
  );

  function toggleMore() {
    setMoreOpen(!moreOpen);
    try {
      localStorage.setItem("navMore", moreOpen ? "0" : "1");
    } catch {}
  }

  const item = (p: Page, className = "nav-item") => (
    <button key={p} className={className} aria-current={page === p ? "page" : undefined} onClick={() => setPage(p)}>
      {t(`nav.${p}` as Key)}
    </button>
  );

  useEffect(() => {
    isElevated().then(setElevated, () => {});
    onedriveRoots().then((r) => setHasOneDrive(r.length > 0), () => {});
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
        {MAIN.map((p) => item(p))}
        <button className="nav-item nav-group" aria-expanded={moreOpen} onClick={toggleMore}>
          <span>{t("nav.more")}</span>
          <span className="chevron" aria-hidden>
            {moreOpen ? "▾" : "▸"}
          </span>
        </button>
        {/* La page ouverte reste visible même si le groupe est replié. */}
        {more.filter((p) => moreOpen || page === p).map((p) => item(p, "nav-item nav-sub"))}
        <div className="nav-footer">
          {item("about", "nav-item nav-small")}
          <button
            className="nav-item nav-icon"
            aria-current={page === "settings" ? "page" : undefined}
            aria-label={t("nav.settings")}
            title={t("nav.settings")}
            onClick={() => setPage("settings")}
          >
            <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h0a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h0a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v0a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </button>
        </div>
      </nav>
      <main className="content">
        <UpdateBanner />
        <h1>{t(`nav.${page}` as Key)}</h1>
        {page === "home" && (
          <Home
            onScan={() => {
              setScanRequest((n) => n + 1);
              setPage("analyse");
            }}
            go={setPage}
          />
        )}
        {page === "onedrive" && <OneDrive />}
        {/* Analyse reste montée : le résultat du scan survit aux changements d'onglet. */}
        <div className="page" hidden={page !== "analyse"}>
          <Analyse onGoCleanup={() => setPage("cleanup")} scanRequest={scanRequest} />
        </div>
        {page === "media" && <Media />}
        {page === "duplicates" && <Duplicates />}
        {page === "projects" && <Projects />}
        {page === "journal" && <Journal />}
        {page === "settings" && <Settings />}
        {page === "about" && <About />}
        {page === "cleanup" && <Cleanup />}
        {page === "programs" && <Programs />}
      </main>
    </div>
  );
}

export default App;
