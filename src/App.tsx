import { useEffect, useState } from "react";
import { StatusPill } from "./components/StatusPill";
import { Disclaimer, isDisclaimerAccepted } from "./components/Disclaimer";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { LangMenu } from "./components/LangMenu";
import { NavIcon } from "./components/NavIcon";
import { isSetupDone, Setup } from "./components/Setup";
import { applyPendingAutostart } from "./lib/autostart";
import { UpdateBanner } from "./components/UpdateBanner";
import { useT, type Key } from "./i18n/useT";
import { isElevated, onedriveRoots } from "./lib/api";
import { useScanResult } from "./lib/scanStore";
import { About } from "./pages/About";
import { Analyse } from "./pages/Analyse";
import { Cleanup } from "./pages/Cleanup";
import { Duplicates } from "./pages/Duplicates";
import { Health } from "./pages/Health";
import { Home } from "./pages/Home";
import { OneDrive } from "./pages/OneDrive";
import { Journal } from "./pages/Journal";
import { Media } from "./pages/Media";
import { Programs } from "./pages/Programs";
import { Projects } from "./pages/Projects";
import { Settings } from "./pages/Settings";
import { Shortcuts } from "./pages/Shortcuts";
import { Startup } from "./pages/Startup";
import { Updates } from "./pages/Updates";

// L'essentiel, toujours visible ; outils avancés repliables ; pages secondaires en bas.
const MAIN = ["home", "analyse", "cleanup", "media", "programs", "startup", "health", "updates"] as const;
const MORE = ["duplicates", "projects", "shortcuts", "onedrive", "journal"] as const;
type Page = (typeof MAIN)[number] | (typeof MORE)[number] | "settings" | "about";

/** Mode « demo » : la page vient de l'adresse (?page=health), pour les captures d'écran. */
function demoPage(): Page {
  return (new URLSearchParams(location.search).get("page") as Page | null) ?? "home";
}

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
  // Le mode demo (captures) saute l'assistant.
  const [setupDone, setSetupDone] = useState(import.meta.env.MODE === "demo" || isSetupDone());
  const [page, setPage] = useState<Page>(import.meta.env.MODE === "demo" ? demoPage() : "home");
  // Incrémenté par l'Accueil : l'Analyse lance alors une analyse.
  const [scanRequest, setScanRequest] = useState(import.meta.env.MODE === "demo" && new URLSearchParams(location.search).has("autoscan") ? 1 : 0);
  const [hasOneDrive, setHasOneDrive] = useState(false);
  const [elevated, setElevated] = useState(false);
  const [moreOpen, setMoreOpen] = useState(savedMoreOpen);
  const [quickClean, setQuickClean] = useState(false);
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

  const item = (p: Page, className = "nav-item") => {
    const n = (MAIN as readonly string[]).indexOf(p);
    return (
      <button key={p} className={className} aria-current={page === p ? "page" : undefined} title={n >= 0 ? `Ctrl+${n + 1}` : undefined} onClick={() => setPage(p)}>
        <NavIcon page={p} />
        {t(`nav.${p}` as Key)}
      </button>
    );
  };

  // Raccourcis : Ctrl+1…8 pour les pages principales, F5 relance l'analyse (au lieu de recharger la fenêtre).
  const ready = accepted && setupDone;
  useEffect(() => {
    if (!ready) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.altKey && !e.shiftKey && /^[1-8]$/.test(e.key)) {
        e.preventDefault();
        setPage(MAIN[Number(e.key) - 1]);
      } else if (e.key === "F5" || (e.ctrlKey && e.key.toLowerCase() === "r")) {
        e.preventDefault();
        setScanRequest((n) => n + 1);
        setPage("analyse");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [ready]);

  useEffect(() => {
    isElevated().then((e) => {
      setElevated(e);
      // Choix « démarrer avec Windows » fait avant la relance en admin.
      if (e) applyPendingAutostart().catch(() => {});
    }, () => {});
    onedriveRoots().then((r) => setHasOneDrive(r.length > 0), () => {});
  }, []);

  if (!accepted) return <Disclaimer onAccept={() => setAccepted(true)} />;
  if (!setupDone) return <Setup elevated={elevated} onDone={() => setSetupDone(true)} />;

  return (
    <div className="shell">
      <nav className="sidebar">
        <div className="brand">
          <span className="brand-logo" aria-hidden />
          Kleaner
        </div>
        {elevated && (
          <div className="brand-badge">
            <StatusPill status="ok">{t("admin.badge")}</StatusPill>
          </div>
        )}
        {/* Nettoyage : l'action principale, mise en avant comme un bouton. */}
        {MAIN.map((p) => item(p, p === "cleanup" ? "nav-item nav-cta" : "nav-item"))}
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
          <LangMenu />
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
        <ErrorBoundary resetKey={page}>
          {page === "home" && (
            <Home
              onScan={() => {
                setScanRequest((n) => n + 1);
                setPage("analyse");
              }}
              go={setPage}
              onQuickClean={() => {
                setQuickClean(true);
                setPage("cleanup");
              }}
            />
          )}
          {page === "onedrive" && <OneDrive />}
          {page === "updates" && <Updates />}
          {page === "startup" && <Startup />}
          {page === "health" && <Health />}
          {page === "shortcuts" && <Shortcuts />}
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
          {page === "cleanup" && <Cleanup quick={quickClean} onQuickDone={() => setQuickClean(false)} />}
          {page === "programs" && <Programs />}
        </ErrorBoundary>
      </main>
    </div>
  );
}

export default App;
