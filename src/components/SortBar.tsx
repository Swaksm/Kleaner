import { useT } from "../i18n/useT";
import { nextSort, type SortState } from "../lib/sort";

/** Boutons « Trier par » ; la clé active affiche son sens (▲ croissant, ▼ décroissant). */
export function SortBar<K extends string>({
  options,
  value,
  onChange,
}: {
  /** clé → [libellé, est numérique] */
  options: Record<K, [string, boolean]>;
  value: SortState<K>;
  onChange: (next: SortState<K>) => void;
}) {
  const { t } = useT();
  return (
    <div className="row sortbar" role="group" aria-label={t("sort.by")}>
      <span className="muted">{t("sort.by")}</span>
      {(Object.keys(options) as K[]).map((k) => {
        const [label, numeric] = options[k];
        const active = value.key === k;
        return (
          <button
            key={k}
            type="button"
            className={active ? "btn btn-small btn-active" : "btn btn-small"}
            aria-pressed={active}
            onClick={() => onChange(nextSort(value, k, numeric))}
          >
            {label}
            {active && (value.dir === "asc" ? " ▲" : " ▼")}
          </button>
        );
      })}
    </div>
  );
}
