import { useState, type ReactNode } from "react";

/**
 * Carte de page. Avec `fold`, elle se replie sur son titre et un résumé (taille, nombre trouvé) :
 * les pages longues tiennent sur un écran, et l'état ouvert ou fermé est retenu.
 */
export function Card({ title, children, fold }: { title?: string; children: ReactNode; fold?: { id: string; summary?: ReactNode; open?: boolean } }) {
  const key = fold && `fold:${fold.id}`;
  const [open, setOpen] = useState(() => {
    try {
      const saved = key ? localStorage.getItem(key) : null;
      return saved === null ? (fold?.open ?? false) : saved === "1";
    } catch {
      return fold?.open ?? false;
    }
  });
  if (!fold) {
    return (
      <section className="card">
        {title && <h2>{title}</h2>}
        {children}
      </section>
    );
  }
  return (
    <details
      className="card fold"
      open={open}
      onToggle={(e) => {
        const now = e.currentTarget.open;
        setOpen(now);
        try {
          localStorage.setItem(key!, now ? "1" : "0");
        } catch {}
      }}
    >
      <summary>
        <h2>{title}</h2>
        {fold.summary != null && <span className="fold-summary muted">{fold.summary}</span>}
      </summary>
      {children}
    </details>
  );
}
