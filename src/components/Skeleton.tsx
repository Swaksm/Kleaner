/** Blocs gris animés pendant un chargement (à la place de « … »). */
export function Skeleton({ lines = 3 }: { lines?: number }) {
  return (
    <div className="skeleton-block" role="status" aria-busy="true" aria-label="…">
      {Array.from({ length: lines }, (_, i) => (
        <span key={i} className="skeleton" style={{ width: `${90 - (i % 3) * 18}%` }} />
      ))}
    </div>
  );
}

/** Variante en ligne : une valeur pas encore connue (taille, verdict). */
export function SkeletonInline() {
  return <span className="skeleton skeleton-inline" aria-label="…" />;
}
