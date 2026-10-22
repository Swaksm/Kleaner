/** Remplissage d'un disque : couleur selon le niveau (au-delà de 75 % puis de 90 %). */
function usageTone(ratio: number): string {
  return ratio >= 0.9 ? " fill-danger" : ratio >= 0.75 ? " fill-warn" : " fill-ok";
}

/** Ligne « libellé — valeur » avec une barre proportionnelle (thème géré en CSS). `usage` : barre d'un disque, colorée. */
export function SizeBar({ label, value, ratio, usage = false }: { label: string; value: string; ratio: number; usage?: boolean }) {
  return (
    <div className="sizebar">
      <div className="sizebar-head">
        <span>{label}</span>
        <span className="num">{value}</span>
      </div>
      <div className="sizebar-track" aria-hidden>
        <div className={"sizebar-fill" + (usage ? usageTone(ratio) : "")} style={{ width: `${Math.max(0, Math.min(1, ratio)) * 100}%` }} />
      </div>
    </div>
  );
}
