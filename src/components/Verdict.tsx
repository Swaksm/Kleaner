import { useEffect, useRef } from "react";
import { useT, type Key } from "../i18n/useT";
import { revealInExplorer, type Explanation, type Verdict } from "../lib/api";
import { StatusPill } from "./StatusPill";

const STATUS: Record<Verdict, "ok" | "warn" | "danger" | "neutral"> = {
  safe: "ok",
  cleanup: "ok",
  uninstall: "warn",
  personal: "warn",
  breaks: "danger",
  system: "danger",
  unknown: "neutral",
};

const ICON: Record<Verdict, string> = {
  safe: "✓",
  cleanup: "🧹",
  uninstall: "📦",
  personal: "👤",
  breaks: "⚠",
  system: "⛔",
  unknown: "?",
};

/** Badge cliquable : icône + libellé (jamais la couleur seule). */
export function VerdictPill({ e, onOpen }: { e?: Explanation; onOpen: (e: Explanation) => void }) {
  const { t } = useT();
  if (!e) return <span className="verdict-pill muted">…</span>;
  return (
    <button type="button" className="verdict-pill" title={t("verdict.open")} onClick={() => onOpen(e)}>
      <StatusPill status={STATUS[e.verdict]}>
        <span aria-hidden>{ICON[e.verdict]}</span> {t(`verdict.${e.verdict}` as Key)}
      </StatusPill>
    </button>
  );
}

/** Fiche « Qu'est-ce que c'est ? ». */
export function ExplainDialog({ e, onClose }: { e: Explanation; onClose: () => void }) {
  const { t, tl } = useT();
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => ref.current?.showModal(), []);
  return (
    <dialog
      ref={ref}
      className="dialog"
      onCancel={onClose}
      onClick={(ev) => ev.target === ref.current && onClose()}
    >
      <h2>{t("verdict.title")}</h2>
      <div className="dialog-body">
        <code>{e.path}</code>
        <StatusPill status={STATUS[e.verdict]}>
          <span aria-hidden>{ICON[e.verdict]}</span> {t(`verdict.${e.verdict}` as Key)}
        </StatusPill>
        <dl className="cleaner-info">
          <dt>{t("verdict.what")}</dt>
          <dd>{e.what[tl]}</dd>
          <dt>{t("verdict.consequence")}</dt>
          <dd>{e.consequence[tl]}</dd>
          <dt>{t("verdict.howto")}</dt>
          <dd>{e.howto[tl]}</dd>
        </dl>
        <p className="muted small">{t(`verdict.source.${e.source}` as Key)}</p>
      </div>
      <div className="dialog-actions">
        <button className="btn" onClick={() => revealInExplorer(e.path)}>
          {t("analyse.open")}
        </button>
        <button className="btn btn-primary" onClick={onClose}>
          {t("report.close")}
        </button>
      </div>
    </dialog>
  );
}
