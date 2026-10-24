import { useEffect, useRef, useState } from "react";
import { LANGS, setLang, useT, type Lang } from "../i18n/useT";

// Drapeaux en SVG : Windows n'affiche pas les drapeaux emoji (il écrit « FR »).
const bands = (dir: "v" | "h", colors: string[], weights = colors.map(() => 1)) => {
  const total = weights.reduce((a, b) => a + b, 0);
  let at = 0;
  return colors.map((c, i) => {
    const size = (weights[i] / total) * (dir === "v" ? 30 : 20);
    const r = dir === "v" ? <rect key={i} x={at} y={0} width={size} height={20} fill={c} /> : <rect key={i} x={0} y={at} width={30} height={size} fill={c} />;
    at += size;
    return r;
  });
};

const FLAGS: Record<Lang, React.ReactNode> = {
  en: (
    <>
      <rect width="30" height="20" fill="#012169" />
      <path d="M0 0l30 20M30 0L0 20" stroke="#fff" strokeWidth="4" />
      <path d="M0 0l30 20M30 0L0 20" stroke="#C8102E" strokeWidth="1.5" />
      <path d="M15 0v20M0 10h30" stroke="#fff" strokeWidth="6" />
      <path d="M15 0v20M0 10h30" stroke="#C8102E" strokeWidth="3.5" />
    </>
  ),
  fr: bands("v", ["#0055A4", "#fff", "#EF4135"]),
  it: bands("v", ["#009246", "#fff", "#CE2B37"]),
  de: bands("h", ["#000", "#DD0000", "#FFCE00"]),
  es: bands("h", ["#AA151B", "#F1BF00", "#AA151B"], [1, 2, 1]),
  pt: bands("v", ["#046A38", "#DA291C"], [2, 3]),
  ru: bands("h", ["#fff", "#0039A6", "#D52B1E"]),
  zh: (
    <>
      <rect width="30" height="20" fill="#EE1C25" />
      <path d="M5 2.2l.9 2.8h2.9l-2.4 1.7.9 2.8L5 7.8 2.7 9.5l.9-2.8-2.4-1.7h2.9z" fill="#FFFF00" />
    </>
  ),
};

export function Flag({ lang }: { lang: Lang }) {
  return (
    <svg className="flag" viewBox="0 0 30 20" width="21" height="14" aria-hidden>
      {FLAGS[lang]}
    </svg>
  );
}

/** Bouton drapeau à côté de l'engrenage : liste des langues au clic. */
export function LangMenu() {
  const { t, lang } = useT();
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent | KeyboardEvent) => {
      if (e instanceof KeyboardEvent ? e.key === "Escape" : !box.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", close);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", close);
    };
  }, [open]);

  return (
    <div className="lang-menu" ref={box}>
      <button className="nav-item nav-icon" aria-haspopup="menu" aria-expanded={open} aria-label={t("settings.language")} title={t("settings.language")} onClick={() => setOpen(!open)}>
        <Flag lang={lang} />
      </button>
      {open && (
        <div className="lang-list" role="menu">
          {LANGS.map(([l, name]) => (
            <button
              key={l}
              role="menuitemradio"
              aria-checked={l === lang}
              className="lang-option"
              onClick={() => {
                setLang(l);
                setOpen(false);
              }}
            >
              <Flag lang={l} />
              {name}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
