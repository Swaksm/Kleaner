import { useT } from "../i18n/useT";
import type { Item } from "../lib/api";
import { formatBytes } from "../lib/format";
import { ConfirmDialog } from "./ConfirmDialog";

/** Confirmation d'envoi à la Corbeille : nombre, taille, mode et liste exacte des chemins. */
export function TrashConfirm({
  items,
  onConfirm,
  onCancel,
}: {
  items: Item[];
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { t, lang } = useT();
  const total = items.reduce((s, i) => s + i.size, 0);
  return (
    <ConfirmDialog
      open={items.length > 0}
      title={t("trash.title")}
      confirmLabel={t("trash.confirm")}
      cancelLabel={t("common.cancel")}
      onConfirm={onConfirm}
      onCancel={onCancel}
    >
      <p>
        <span className="num">{items.length}</span> {t("trash.files")} ·{" "}
        <span className="num">{formatBytes(total, lang)}</span>
      </p>
      <p className="ok">{t("trash.mode")}</p>
      <p className="muted">{t("trash.space")}</p>
      <ul className="paths">
        {items.map((i) => (
          <li key={i.path}>{i.path}</li>
        ))}
      </ul>
    </ConfirmDialog>
  );
}
