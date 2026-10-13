import { useEffect, useRef, useState } from "react";
import { useT } from "../i18n/useT";
import {
  estimateCleaners,
  listCleaners,
  listPrograms,
  programSizes,
  revealInExplorer,
  saveReport,
  type ScanResult,
} from "../lib/api";
import { buildReport } from "../lib/report";
import { StatusPill } from "./StatusPill";

/** Génère le rapport pour une IA, l'affiche en aperçu, puis copie ou enregistre. Rien n'est envoyé. */
export function ReportDialog({ scan, onClose }: { scan: ScanResult; onClose: () => void }) {
  const { t, lang } = useT();
  const ref = useRef<HTMLDialogElement>(null);
  const [content, setContent] = useState<string | null>(null);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  useEffect(() => {
    ref.current?.showModal();
    Promise.all([listCleaners(), estimateCleaners(), listPrograms(), programSizes()])
      .then(([view, estimates, list, sizes]) => {
        // Taille réelle mesurée quand elle est connue, sinon taille déclarée.
        const real = new Map(sizes.map((s) => [s.id, s.bytes]));
        const programs = list
          .map((p) => ({ ...p, bytes: real.get(p.id) ?? p.bytes }))
          .sort((a, b) => b.bytes - a.bytes);
        setContent(buildReport({ scan, cleaners: view.cleaners, estimates, elevated: view.elevated, programs, lang }));
      })
      .catch((e) => setMessage({ status: "danger", text: String(e) }));
  }, [scan, lang]);

  async function copy() {
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      setMessage({ status: "ok", text: t("report.copied") });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  async function save() {
    if (!content) return;
    try {
      const path = await saveReport(content);
      setMessage({ status: "ok", text: t("report.saved") });
      await revealInExplorer(path);
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  return (
    <dialog ref={ref} className="dialog dialog-wide" onCancel={onClose}>
      <h2>{t("report.title")}</h2>
      <div className="dialog-body">
        <p>{t("report.intro")}</p>
        <p className="warn">{t("report.privacy")}</p>
        <textarea className="report-preview" readOnly value={content ?? t("report.loading")} />
        {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      </div>
      <div className="dialog-actions">
        <button className="btn" onClick={onClose}>
          {t("report.close")}
        </button>
        <button className="btn" disabled={!content} onClick={save}>
          {t("report.save")}
        </button>
        <button className="btn btn-primary" disabled={!content} onClick={copy}>
          {t("report.copy")}
        </button>
      </div>
    </dialog>
  );
}
