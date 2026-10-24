import type { ReactNode } from "react";
import { useT } from "../i18n/useT";
import type { CleanReport, Cleaner } from "../lib/api";
import { formatBytes } from "../lib/format";
import { SkeletonInline } from "./Skeleton";
import { StatusPill } from "./StatusPill";

/** Une entrée du catalogue : ce que ça supprime, pourquoi c'est inutile, ce que ça change. */
export function CleanerCard({
  cleaner,
  report,
  locked,
  checked,
  onToggle,
  compact,
  children,
}: {
  cleaner: Cleaner;
  report?: CleanReport;
  /** Raison pour laquelle l'entrée ne peut pas être cochée (admin, appli ouverte…). */
  locked?: string;
  checked?: boolean;
  onToggle?: () => void;
  /** Catalogue communautaire : le texte est le même partout, on montre seulement les dossiers vidés. */
  compact?: boolean;
  children?: ReactNode;
}) {
  const { t, lang, tx } = useT();
  const size =
    report === undefined
      ? <SkeletonInline />
      : cleaner.id === "component-store"
        ? t("cleanup.sizeUnknown")
        : formatBytes(report.bytes, lang);

  return (
    <div className="cleaner">
      <div className="cleaner-head">
        {onToggle ? (
          <label className="check">
            <input type="checkbox" checked={!!checked} disabled={!!locked} onChange={onToggle} />
            <strong>{tx(cleaner.label)}</strong>
          </label>
        ) : (
          <strong>{tx(cleaner.label)}</strong>
        )}
        <span className="num">{size}</span>
      </div>
      {compact && (
        <span className="muted path small" title={cleaner.only.length ? cleaner.only.join(", ") : undefined}>
          {cleaner.paths.join(" · ")}
        </span>
      )}
      <dl className="cleaner-info" hidden={compact}>
        <dt>{t("cleanup.what")}</dt>
        <dd>{tx(cleaner.what)}</dd>
        <dt>{t("cleanup.why")}</dt>
        <dd>{tx(cleaner.why)}</dd>
        <dt>{t("cleanup.effect")}</dt>
        <dd>{tx(cleaner.effect)}</dd>
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
