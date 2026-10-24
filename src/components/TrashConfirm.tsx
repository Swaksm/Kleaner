import { useEffect, useState } from "react";
import { useT, type Key } from "../i18n/useT";
import { isCritical, type Explanation, type Item } from "../lib/api";
import { formatBytes } from "../lib/format";
import { ConfirmDialog } from "./ConfirmDialog";

/**
 * Confirmation d'envoi à la Corbeille : nombre, taille, mode et liste exacte des chemins.
 * Si un fichier est critique (casse un logiciel, système, à désinstaller), il est signalé
 * et l'utilisateur doit cocher qu'il a compris : on avertit, on n'empêche pas.
 */
export function TrashConfirm({
  items,
  explanations,
  onConfirm,
  onCancel,
}: {
  items: Item[];
  explanations?: Map<string, Explanation>;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { t, lang, tx } = useT();
  const [ack, setAck] = useState(false);
  const total = items.reduce((s, i) => s + i.size, 0);
  const critical = items.filter((i) => {
    const e = explanations?.get(i.path);
    return e && isCritical(e.verdict);
  });

  useEffect(() => setAck(false), [items]);

  return (
    <ConfirmDialog
      open={items.length > 0}
      title={t("trash.title")}
      confirmLabel={t("trash.confirm")}
      cancelLabel={t("common.cancel")}
      danger={critical.length > 0}
      confirmDisabled={critical.length > 0 && !ack}
      onConfirm={onConfirm}
      onCancel={onCancel}
    >
      <p>
        <span className="num">{items.length}</span> {t("trash.files")} ·{" "}
        <span className="num">{formatBytes(total, lang)}</span>
      </p>
      <p className="ok">{t("trash.mode")}</p>
      <p className="muted">{t("trash.space")}</p>
      {critical.length > 0 && (
        <div className="critical">
          <p className="danger">
            <strong>{t("trash.criticalTitle")}</strong>
          </p>
          <ul>
            {critical.map((i) => {
              const e = explanations!.get(i.path)!;
              return (
                <li key={i.path}>
                  <strong>{t(`verdict.${e.verdict}` as Key)}</strong> — {tx(e.consequence)}
                  <br />
                  <span className="muted small">{tx(e.howto)}</span>
                </li>
              );
            })}
          </ul>
          <label className="check">
            <input type="checkbox" checked={ack} onChange={(ev) => setAck(ev.target.checked)} />
            {t("trash.criticalAck")}
          </label>
        </div>
      )}
      <ul className="paths">
        {items.map((i) => (
          <li key={i.path}>{i.path}</li>
        ))}
      </ul>
    </ConfirmDialog>
  );
}
