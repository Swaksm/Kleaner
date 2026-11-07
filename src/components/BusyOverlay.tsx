import { useEffect, useRef } from "react";
import { useT } from "../i18n/useT";
import { onActionProgress } from "../lib/api";
import { requestStop, setProgress, useBusy } from "../lib/busy";

/** Fenêtre d'attente au milieu, fond grisé : rien d'autre n'est cliquable tant que l'action tourne. */
export function BusyOverlay() {
  const { t } = useT();
  const busy = useBusy();
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const off = onActionProgress((p) => setProgress(p.done, p.total, p.label));
    return () => {
      off.then((f) => f()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    const d = ref.current;
    if (!d) return;
    if (busy && !d.open) d.showModal();
    if (!busy && d.open) d.close();
  }, [busy]);

  const label = busy?.label ? (busy.name?.(busy.label) ?? busy.label) : "";
  return (
    <dialog
      ref={ref}
      className="dialog busy"
      aria-busy="true"
      // Échap ne ferme pas : l'action continuerait derrière.
      onCancel={(e) => e.preventDefault()}
    >
      {busy && (
        <>
          <h2>{busy.stopping ? t("busy.stopping") : t("busy.title")}</h2>
          <div className="dialog-body">
            {busy.total > 0 ? (
              <>
                <progress max={busy.total} value={busy.done} />
                <span className="muted small num">
                  {busy.done + 1} / {busy.total} · {label}
                </span>
              </>
            ) : (
              <progress />
            )}
            <p className="muted small">{busy.onStop ? t("busy.hintStop") : t("busy.hint")}</p>
          </div>
          {busy.onStop && (
            <div className="dialog-actions">
              <button className="btn" disabled={busy.stopping} onClick={requestStop}>
                {t("busy.stop")}
              </button>
            </div>
          )}
        </>
      )}
    </dialog>
  );
}
