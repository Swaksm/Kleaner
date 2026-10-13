import { useEffect, useRef, useState } from "react";
import { useT } from "../i18n/useT";
import { previewCleaners, revealInExplorer, type Cleaner, type PreviewView } from "../lib/api";
import { formatBytes } from "../lib/format";
import { outside } from "./MediaViewer";
import { StatusPill } from "./StatusPill";

/** Simulation : la liste exacte des fichiers qui seraient supprimés. Rien n'est supprimé. */
export function CleanupPreview({ cleaners, onClose }: { cleaners: Cleaner[]; onClose: () => void }) {
  const { t, lang, tl } = useT();
  const ref = useRef<HTMLDialogElement>(null);
  const [view, setView] = useState<PreviewView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const byId = new Map(cleaners.map((c) => [c.id, c]));

  useEffect(() => {
    ref.current?.showModal();
    previewCleaners(cleaners.map((c) => c.id)).then(setView, (e) => setError(String(e)));
  }, []);

  return (
    <dialog ref={ref} className="dialog dialog-wide" onCancel={onClose} onClick={(e) => outside(e, ref.current) && onClose()}>
      <h2>{t("preview.title")}</h2>
      <div className="dialog-body">
        <p className="ok">{t("preview.intro")}</p>
        {!view && !error && (
          <span className="loading" role="status">
            <span className="spinner" aria-hidden /> {t("cleanup.estimating")}
          </span>
        )}
        {error && <StatusPill status="danger">{error}</StatusPill>}
        {view?.entries.map((e) => {
          const c = byId.get(e.id);
          return (
            <details key={e.id} className="preview-entry">
              <summary>
                <strong>{c?.label[tl] ?? e.id}</strong> · <span className="num">{e.files.toLocaleString(lang)}</span>{" "}
                {t("analyse.files")} · <span className="num">{formatBytes(e.bytes, lang)}</span>
              </summary>
              {c?.command && <code>{c.command}</code>}
              {e.sample.length > 0 && (
                <ul className="paths">
                  {e.sample.map((f) => (
                    <li key={f.path}>
                      {formatBytes(f.size, lang)} — {f.path}
                    </li>
                  ))}
                  {e.files > e.sample.length && <li className="muted">… +{(e.files - e.sample.length).toLocaleString(lang)}</li>}
                </ul>
              )}
            </details>
          );
        })}
      </div>
      <div className="dialog-actions">
        {view && (
          <button className="btn" onClick={() => revealInExplorer(view.file)}>
            {t("preview.openFull")}
          </button>
        )}
        <button className="btn btn-primary" onClick={onClose}>
          {t("report.close")}
        </button>
      </div>
    </dialog>
  );
}
