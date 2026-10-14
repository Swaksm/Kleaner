import { useState } from "react";
import { useT } from "../i18n/useT";
import { runCleaners } from "../lib/api";
import { formatBytes } from "../lib/format";
import { ConfirmDialog } from "./ConfirmDialog";
import { StatusPill } from "./StatusPill";

/** Après une mise à la Corbeille : la place n'est libérée qu'en la vidant. Irréversible, donc confirmé. */
export function EmptyBin() {
  const { t, lang } = useT();
  const [confirm, setConfirm] = useState(false);
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  async function empty() {
    setConfirm(false);
    setRunning(true);
    try {
      const [r] = await runCleaners(["recycle-bin"]);
      setResult(
        r?.error
          ? { status: "danger", text: r.error }
          : { status: "ok", text: `${formatBytes(r?.bytes ?? 0, lang)} ${t("cleanup.freed")}` },
      );
    } catch (e) {
      setResult({ status: "danger", text: String(e) });
    } finally {
      setRunning(false);
    }
  }

  if (result) return <StatusPill status={result.status}>{result.text}</StatusPill>;
  return (
    <>
      <button className="btn btn-small" disabled={running} onClick={() => setConfirm(true)}>
        {running ? t("cleanup.running") : t("bin.empty")}
      </button>
      <ConfirmDialog
        open={confirm}
        title={`${t("bin.empty")} ?`}
        confirmLabel={t("bin.empty")}
        cancelLabel={t("common.cancel")}
        danger
        onConfirm={empty}
        onCancel={() => setConfirm(false)}
      >
        <p className="danger">{t("bin.confirm")}</p>
      </ConfirmDialog>
    </>
  );
}
