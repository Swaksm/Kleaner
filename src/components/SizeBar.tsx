/** Ligne « libellé — valeur » avec une barre proportionnelle (thème géré en CSS). */
export function SizeBar({ label, value, ratio }: { label: string; value: string; ratio: number }) {
  return (
    <div className="sizebar">
      <div className="sizebar-head">
        <span>{label}</span>
        <span className="num">{value}</span>
      </div>
      <div className="sizebar-track" aria-hidden>
        <div className="sizebar-fill" style={{ width: `${Math.max(0, Math.min(1, ratio)) * 100}%` }} />
      </div>
    </div>
  );
}
