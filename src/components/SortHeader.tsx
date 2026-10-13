import { useT } from "../i18n/useT";
import { nextSort, type SortState } from "../lib/sort";

/** Une cellule d'en-tête : zéro, une ou plusieurs clés de tri (ex. « Nom » et « Dossier »). */
export type HeaderCell<K extends string> = { sorts: [key: K, label: string, numeric: boolean][]; end?: boolean } | null;

/**
 * Ligne d'en-tête d'une liste `.items` : cliquer un titre de colonne trie, recliquer inverse le sens.
 * Elle doit avoir autant de cellules que les lignes de la liste, pour rester alignée.
 */
export function SortHeader<K extends string>({
  cells,
  value,
  onChange,
}: {
  cells: HeaderCell<K>[];
  value: SortState<K>;
  onChange: (next: SortState<K>) => void;
}) {
  const { t } = useT();
  return (
    <li className="list-header">
      {cells.map((cell, i) => (
        <span key={i} className={cell?.end ? "header-cell end" : "header-cell"}>
          {cell?.sorts.map(([key, label, numeric]) => {
            const active = value.key === key;
            return (
              <button
                key={key}
                type="button"
                className={active ? "sort-col active" : "sort-col"}
                aria-pressed={active}
                aria-label={`${t("sort.by")} ${label}`}
                onClick={() => onChange(nextSort(value, key, numeric))}
              >
                {label}
                <span className="sort-arrow" aria-hidden>
                  {active ? (value.dir === "asc" ? "▲" : "▼") : "↕"}
                </span>
              </button>
            );
          })}
        </span>
      ))}
    </li>
  );
}
