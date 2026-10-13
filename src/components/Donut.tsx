import { useState } from "react";

export type Slice = {
  key: string;
  label: string;
  value: number;
  /** Texte affiché pour la valeur (ex. « 12,3 Go »). */
  display: string;
  /** Emplacement de couleur 1..6, fixé par l'entité (jamais par le rang). */
  slot: number;
};

const R = 70;
const STROKE = 24;
const C = 2 * Math.PI * R;
const GAP = 2; // espace de 2 px entre les segments

/**
 * Anneau + légende. La légende porte libellé, valeur et pourcentage : la couleur n'est jamais seule.
 * Survol ou focus d'un segment / d'une ligne de légende : mise en avant + détail au centre.
 */
export function Donut({ slices, centerLabel, centerValue }: { slices: Slice[]; centerLabel: string; centerValue: string }) {
  const [active, setActive] = useState<string | null>(null);
  const total = slices.reduce((s, x) => s + x.value, 0);
  const shown = slices.filter((s) => s.value > 0);
  const pct = (v: number) => (total > 0 ? Math.round((v / total) * 1000) / 10 : 0);
  const focus = shown.find((s) => s.key === active);

  let offset = 0;
  const arcs = shown.map((s) => {
    const len = total > 0 ? (s.value / total) * C : 0;
    const arc = { ...s, dash: Math.max(0, len - (shown.length > 1 ? GAP : 0)), start: offset };
    offset += len;
    return arc;
  });

  return (
    <div className="donut">
      <svg
        viewBox="0 0 180 180"
        className="donut-svg"
        role="img"
        aria-label={shown.map((s) => `${s.label} ${s.display} (${pct(s.value)} %)`).join(", ")}
      >
        <circle cx="90" cy="90" r={R} className="donut-track" strokeWidth={STROKE} fill="none" />
        {arcs.map((a) => (
          <circle
            key={a.key}
            cx="90"
            cy="90"
            r={R}
            fill="none"
            strokeWidth={active === a.key ? STROKE + 6 : STROKE}
            className={`donut-arc series-${a.slot}`}
            strokeDasharray={`${a.dash} ${C - a.dash}`}
            strokeDashoffset={-a.start}
            transform="rotate(-90 90 90)"
            opacity={active && active !== a.key ? 0.35 : 1}
            onMouseEnter={() => setActive(a.key)}
            onMouseLeave={() => setActive(null)}
          >
            <title>{`${a.label} : ${a.display} (${pct(a.value)} %)`}</title>
          </circle>
        ))}
        <text x="90" y="84" textAnchor="middle" className="donut-center-label">
          {focus ? focus.label : centerLabel}
        </text>
        <text x="90" y="104" textAnchor="middle" className="donut-center-value">
          {focus ? `${focus.display} · ${pct(focus.value)} %` : centerValue}
        </text>
      </svg>
      <ul className="donut-legend">
        {shown.map((s) => (
          <li key={s.key}>
            <button
              type="button"
              className={active === s.key ? "legend-row active" : "legend-row"}
              onMouseEnter={() => setActive(s.key)}
              onMouseLeave={() => setActive(null)}
              onFocus={() => setActive(s.key)}
              onBlur={() => setActive(null)}
            >
              <span className={`swatch series-${s.slot}`} aria-hidden />
              <span className="legend-label">{s.label}</span>
              <span className="num">{s.display}</span>
              <span className="muted legend-pct">{pct(s.value)} %</span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
