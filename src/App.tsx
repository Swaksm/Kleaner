import { useEffect, useState } from "react";
import { StatusPill } from "./components/StatusPill";
import { Disclaimer, isDisclaimerAccepted } from "./components/Disclaimer";
import { useT, type Key } from "./i18n/useT";
import { isElevated } from "./lib/api";
import { useScanResult } from "./lib/scanStore";
import { About } from "./pages/About";
import { Analyse } from "./pages/Analyse";
import { Cleanup } from "./pages/Cleanup";
import { Duplicates } from "./pages/Duplicates";
import { Journal } from "./pages/Journal";
import { Media } from "./pages/Media";
import { Programs } from "./pages/Programs";
import { Projects } from "./pages/Projects";
import { Settings } from "./pages/Settings";

// L'essentiel, toujours visible ; outils avancés repliables ; pages secondaires en bas.
const MAIN = ["analyse", "cleanup", "media", "programs"] as const;
const MORE = ["duplicates", "projects"] as const;
const FOOTER = ["journal", "settings", "about"] as const;
type Page = (typeof MAIN)[number] | (typeof MORE)[number] | (typeof FOOTER)[number];

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
  const [page, setPage] = useState<Page>("analyse");
  const [elevated, setElevated] = useState(false);
  const [moreOpen, setMoreOpen] = useState(savedMoreOpen);
  const scan = useScanResult();
  // « Vieux projets » n'apparaît que si l'analyse a trouvé des projets de code.
  const more = MORE.filter((p) => p !== "projects" || (scan?.artifacts.length ?? 0) > 0);

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
        <div className="nav-footer">{FOOTER.map((p) => item(p, "nav-item nav-small"))}</div>
      </nav>
      <main className="content">
        <h1>{t(`nav.${page}` as Key)}</h1>
        {/* Analyse reste montée : le résultat du scan survit aux changements d'onglet. */}
        <div className="page" hidden={page !== "analyse"}>
          <Analyse />
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
