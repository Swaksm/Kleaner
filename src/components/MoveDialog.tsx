import { useEffect, useRef, useState } from "react";
import { useT } from "../i18n/useT";
import { cancelMove, moveFolder, onMoveProgress, planMove, revealInExplorer, type MovePlan } from "../lib/api";
import { formatBytes } from "../lib/format";
import { SizeBar } from "./SizeBar";
import { StatusPill } from "./StatusPill";

/** Place gardée libre sur le disque choisi (la même marge que côté Rust). */
const MARGIN = 512 * 1024 ** 2;

type Step =
  | { kind: "checking" }
  | { kind: "ready"; plan: MovePlan }
  | { kind: "moving"; plan: MovePlan; done: number }
  | { kind: "done"; dest: string }
  | { kind: "error"; text: string };

/** Déplacer un gros dossier personnel vers un autre disque : vérifications, choix du disque, copie vérifiée. */
export function MoveDialog({ path, onClose }: { path: string; onClose: (moved: boolean) => void }) {
  const { t, lang } = useT();
  const ref = useRef<HTMLDialogElement>(null);
  const [step, setStep] = useState<Step>({ kind: "checking" });
  const [root, setRoot] = useState("");

  useEffect(() => {
    ref.current?.showModal();
    planMove(path).then(
      (plan) => {
        setStep({ kind: "ready", plan });
        setRoot(plan.targets.find((x) => x.free >= plan.bytes + MARGIN)?.root ?? "");
      },
      (e) => setStep({ kind: "error", text: String(e) }),
    );
    const off = onMoveProgress((p) => setStep((s) => (s.kind === "moving" ? { ...s, done: p.done } : s)));
    return () => {
      off.then((f) => f()).catch(() => {});
    };
  }, [path]);

  async function go(plan: MovePlan) {
    setStep({ kind: "moving", plan, done: 0 });
    try {
      const [, dest] = await moveFolder(path, root);
      setStep({ kind: "done", dest });
    } catch (e) {
      setStep({ kind: "error", text: String(e) });
    }
  }

  const close = () => onClose(step.kind === "done");
  const plan = step.kind === "ready" || step.kind === "moving" ? step.plan : null;
  const target = plan?.targets.find((x) => x.root === root);

  return (
    <dialog
      ref={ref}
      className="dialog"
      // Échap pendant la copie : on ne ferme pas, il y a un bouton Annuler.
      onCancel={(e) => {
        e.preventDefault();
        if (step.kind !== "moving") close();
      }}
    >
      <h2>{t("move.title")}</h2>
      <div className="dialog-body">
        <span className="path">{path}</span>
        {step.kind === "checking" && (
          <span className="loading" role="status">
            <span className="spinner" aria-hidden /> {t("move.checking")}
          </span>
        )}
        {plan && (
          <>
            <span className="num">
              {plan.files.toLocaleString(lang)} {t("analyse.files")} · {formatBytes(plan.bytes, lang)}
            </span>
            <p className="muted small">{t("move.intro")}</p>
            {plan.targets.length === 0 && <StatusPill status="warn">{t("move.noTarget")}</StatusPill>}
            {plan.targets.map((x) => {
              const enough = x.free >= plan.bytes + MARGIN;
              return (
                <label key={x.root} className="check">
                  <input
                    type="radio"
                    name="move-target"
                    checked={root === x.root}
                    disabled={!enough || step.kind === "moving"}
                    onChange={() => setRoot(x.root)}
                  />
                  <strong>{x.root}</strong>
                  <span className="muted small">
                    {formatBytes(x.free, lang)} {t("move.free")}
                    {!enough && ` · ${t("move.noSpace")}`}
                  </span>
                </label>
              );
            })}
            {target && (
              <span className="muted small path">
                {t("move.destIs")} {target.dest}
              </span>
            )}
            <p className="muted small">{t("move.hint")}</p>
          </>
        )}
        {step.kind === "moving" && (
          <SizeBar
            label={t("move.moving")}
            value={`${formatBytes(step.done, lang)} / ${formatBytes(step.plan.bytes, lang)}`}
            ratio={step.done / Math.max(1, step.plan.bytes)}
          />
        )}
        {step.kind === "done" && (
          <>
            <StatusPill status="ok">{t("move.done")}</StatusPill>
            <span className="path">{step.dest}</span>
          </>
        )}
        {step.kind === "error" && <StatusPill status="danger">{step.text}</StatusPill>}
      </div>
      <div className="dialog-actions">
        {step.kind === "moving" ? (
          <button className="btn" onClick={() => cancelMove()}>
            {t("common.cancel")}
          </button>
        ) : (
          <button className="btn" onClick={close}>
            {step.kind === "done" || step.kind === "error" ? t("move.close") : t("common.cancel")}
          </button>
        )}
        {step.kind === "done" && (
          <button className="btn btn-primary" onClick={() => revealInExplorer(step.dest)}>
            {t("analyse.open")}
          </button>
        )}
        {(step.kind === "ready" || step.kind === "moving") && (
          <button className="btn btn-primary" disabled={!target || step.kind === "moving"} onClick={() => go(step.plan)}>
            {t("move.confirm")}
          </button>
        )}
      </div>
    </dialog>
  );
}
