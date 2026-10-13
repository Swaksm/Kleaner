import type { ReactNode } from "react";
import { useT } from "../i18n/useT";
import type { CleanReport, Cleaner } from "../lib/api";
import { formatBytes } from "../lib/format";
import { StatusPill } from "./StatusPill";

/** Une entrée du catalogue : ce que ça supprime, pourquoi c'est inutile, ce que ça change. */
export function CleanerCard({
  cleaner,
  report,
  locked,
  checked,
  onToggle,
  children,
}: {
  cleaner: Cleaner;
  report?: CleanReport;
  /** Raison pour laquelle l'entrée ne peut pas être cochée (admin, appli ouverte…). */
  locked?: string;
  checked?: boolean;
  onToggle?: () => void;
  children?: ReactNode;
}) {
  const { t, lang } = useT();
  const size =
    report === undefined
      ? "…"
      : cleaner.id === "component-store"
        ? t("cleanup.sizeUnknown")
        : formatBytes(report.bytes, lang);

  return (
    <div className="cleaner">
      <div className="cleaner-head">
        {onToggle ? (
          <label className="check">
            <input type="checkbox" checked={!!checked} disabled={!!locked} onChange={onToggle} />
            <strong>{cleaner.label[lang]}</strong>
          </label>
        ) : (
          <strong>{cleaner.label[lang]}</strong>
        )}
        <span className="num">{size}</span>
      </div>
      <dl className="cleaner-info">
        <dt>{t("cleanup.what")}</dt>
        <dd>{cleaner.what[lang]}</dd>
        <dt>{t("cleanup.why")}</dt>
        <dd>{cleaner.why[lang]}</dd>
        <dt>{t("cleanup.effect")}</dt>
        <dd>{cleaner.effect[lang]}</dd>
        {cleaner.command && (
          <>
            <dt>{t("cleanup.command")}</dt>
            <dd>
              <code>{cleaner.command}</code>
            </dd>
          </>
        )}
      </dl>
      <div className="row">
        {locked && <StatusPill status="warn">{locked}</StatusPill>}
        {report && report.files > 0 && (
          <span className="muted">
            <span className="num">{report.files.toLocaleString(lang)}</span> {t("analyse.files")}
          </span>
        )}
        {children}
      </div>
    </div>
  );
}
