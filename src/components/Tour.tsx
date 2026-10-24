import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useT, type Key } from "../i18n/useT";

/**
 * Visite guidée : l'appli est voilée, l'élément présenté est éclairé (halo) et une bulle l'explique.
 * Chaque étape vise les éléments `data-tour="…"` (leur contour commun) ; sans cible, la bulle est au centre.
 */
type Step = { id: string; title: Key; targets?: string[] };

const STEPS: Step[] = [
  { id: "welcome", title: "tour.welcomeTitle" },
  { id: "scan", title: "tour.scanTitle", targets: ["scan"] },
  { id: "analyse", title: "nav.analyse", targets: ["nav-analyse"] },
  { id: "cleanup", title: "nav.cleanup", targets: ["nav-cleanup"] },
  { id: "media", title: "nav.media", targets: ["nav-media"] },
  { id: "programs", title: "nav.programs", targets: ["nav-programs"] },
  { id: "care", title: "tour.careTitle", targets: ["nav-startup", "nav-health", "nav-updates"] },
  { id: "more", title: "nav.more", targets: ["nav-more"] },
  { id: "settings", title: "tour.settingsTitle", targets: ["footer"] },
  { id: "end", title: "tour.endTitle" },
];

const STORAGE_KEY = "tour";
const PAD = 6;
const GAP = 14;
const MARGIN = 16;

/** Visite demandée par l'assistant du premier lancement (gardée si Kleaner se relance en admin entre-temps). */
export function isTourPending(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) === "pending";
  } catch {
    return false;
  }
}

export function requestTour() {
  try {
    if (localStorage.getItem(STORAGE_KEY) !== "done") localStorage.setItem(STORAGE_KEY, "pending");
  } catch {}
}

type Box = { top: number; left: number; width: number; height: number };

function elements(ids: string[]): HTMLElement[] {
  return ids.flatMap((id) => [...document.querySelectorAll<HTMLElement>(`[data-tour="${id}"]`)]).filter((e) => e.getClientRects().length > 0);
}

/** Contour commun des cibles, avec une marge ; `null` si aucune n'est affichée. */
function box(ids: string[] | undefined): Box | null {
  const rects = elements(ids ?? []).map((e) => e.getBoundingClientRect());
  if (rects.length === 0) return null;
  const top = Math.min(...rects.map((r) => r.top)) - PAD;
  const left = Math.min(...rects.map((r) => r.left)) - PAD;
  return { top, left, width: Math.max(...rects.map((r) => r.right)) + PAD - left, height: Math.max(...rects.map((r) => r.bottom)) + PAD - top };
}

/** Bulle à droite de la cible, sinon dessous, dessus, à gauche ; toujours dans la fenêtre. */
export function place(b: Box | null, w: number, h: number, vw: number, vh: number): { left: number; top: number } {
  const x = (v: number) => Math.max(MARGIN, Math.min(v, vw - w - MARGIN));
  const y = (v: number) => Math.max(MARGIN, Math.min(v, vh - h - MARGIN));
  if (!b) return { left: x((vw - w) / 2), top: y((vh - h) / 2) };
  const midY = y(b.top + b.height / 2 - h / 2);
  const midX = x(b.left + b.width / 2 - w / 2);
  if (b.left + b.width + GAP + w <= vw - MARGIN) return { left: b.left + b.width + GAP, top: midY };
  if (b.top + b.height + GAP + h <= vh - MARGIN) return { left: midX, top: b.top + b.height + GAP };
  if (b.top - GAP - h >= MARGIN) return { left: midX, top: b.top - GAP - h };
  return { left: x(b.left - GAP - w), top: midY };
}

export function Tour({ onClose }: { onClose: () => void }) {
  const { t, lang } = useT();
  const [i, setI] = useState(0);
  const [hole, setHole] = useState<Box | null>(null);
  const [view, setView] = useState({ w: window.innerWidth, h: window.innerHeight });
  const [size, setSize] = useState({ w: 340, h: 220 });
  const pop = useRef<HTMLDivElement>(null);
  const primary = useRef<HTMLButtonElement>(null);
  const step = STEPS[i];
  const last = i === STEPS.length - 1;

  function close() {
    try {
      localStorage.setItem(STORAGE_KEY, "done");
    } catch {}
    onClose();
  }

  // Le reste de l'appli ne réagit plus (souris, clavier, Tab) tant que la visite est ouverte.
  useEffect(() => {
    const shell = document.querySelector(".shell");
    shell?.setAttribute("inert", "");
    return () => shell?.removeAttribute("inert");
  }, []);

  // Cible de l'étape : mise en vue, mesurée, suivie si la fenêtre change de taille.
  useLayoutEffect(() => {
    elements(step.targets ?? [])[0]?.scrollIntoView({ block: "center" });
    const measure = () => {
      setHole(box(step.targets));
      setView({ w: window.innerWidth, h: window.innerHeight });
    };
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, [i]);

  // Taille réelle de la bulle (selon le texte et la langue) pour la placer.
  useLayoutEffect(() => {
    const p = pop.current;
    if (p && (p.offsetWidth !== size.w || p.offsetHeight !== size.h)) setSize({ w: p.offsetWidth, h: p.offsetHeight });
  }, [i, hole, lang, size]);

  useEffect(() => primary.current?.focus(), [i]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
      else if (e.key === "ArrowRight" && !last) setI(i + 1);
      else if (e.key === "ArrowLeft" && i > 0) setI(i - 1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [i]);

  // Sans cible, le halo se referme au centre : le voile couvre toute l'appli.
  const h = hole ?? { top: view.h / 2, left: view.w / 2, width: 0, height: 0 };
  const at = place(hole, size.w, size.h, view.w, view.h);

  return (
    <div className="tour">
      <div className={hole ? "tour-hole" : "tour-hole tour-hole-none"} style={h} aria-hidden />
      <div
        key={i}
        ref={pop}
        className="tour-pop"
        role="dialog"
        aria-modal="true"
        aria-labelledby="tour-title"
        aria-describedby="tour-text"
        style={at}
      >
        <span className="tour-count">
          {i + 1} / {STEPS.length}
        </span>
        <h2 id="tour-title">{t(step.title)}</h2>
        <p id="tour-text">{t(`tour.${step.id}` as Key)}</p>
        <div className="tour-dots" aria-hidden>
          {STEPS.map((s, n) => (
            <span key={s.id} className={n === i ? "on" : undefined} />
          ))}
        </div>
        <div className="tour-actions">
          {!last && (
            <button className="link tour-skip" onClick={close}>
              {t("tour.skip")}
            </button>
          )}
          {i > 0 && (
            <button className="btn" onClick={() => setI(i - 1)}>
              {t("tour.prev")}
            </button>
          )}
          <button ref={primary} className="btn btn-primary" onClick={() => (last ? close() : setI(i + 1))}>
            {last ? t("tour.done") : i === 0 ? t("tour.start") : t("tour.next")}
          </button>
        </div>
      </div>
    </div>
  );
}
