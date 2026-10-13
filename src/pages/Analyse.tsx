import { useEffect, useState } from "react";
import { AdminBanner } from "../components/AdminBanner";
import { Card } from "../components/Card";
import { SizeBar } from "../components/SizeBar";
import { StatusPill } from "../components/StatusPill";
import { ReportDialog } from "../components/ReportDialog";
import { TrashConfirm } from "../components/TrashConfirm";
import { useT, type Key } from "../i18n/useT";
import {
  cancelScan,
  listCleaners,
  moveToTrash,
  onScanDone,
  onScanProgress,
  revealInExplorer,
  startScan,
  type Item,
  type ScanProgress,
  type ScanResult,
} from "../lib/api";
import { formatBytes } from "../lib/format";

type Phase =
  | { kind: "idle" }
  | { kind: "scanning"; progress: ScanProgress; start: number }
  | { kind: "done"; result: ScanResult };

type Message = { status: "ok" | "danger"; text: string };

export function Analyse() {
  const { t, lang } = useT();
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState<Item[]>([]);
  const [message, setMessage] = useState<Message | null>(null);
  const [elevated, setElevated] = useState(true);
  const [report, setReport] = useState(false);

  useEffect(() => {
    listCleaners().then((v) => setElevated(v.elevated), () => {});
    const unlisten = [
      onScanProgress((progress) => setPhase((p) => (p.kind === "scanning" ? { ...p, progress } : p))),
      onScanDone((result) => {
        setPhase({ kind: "done", result });
        setSelected(new Set());
      }),
    ];
    unlisten.forEach((u) => u.catch(() => {}));
    return () => unlisten.forEach((u) => u.then((f) => f()).catch(() => {}));
  }, []);

  function start() {
    setMessage(null);
    setPhase({ kind: "scanning", progress: { files: 0, bytes: 0, denied: 0 }, start: Date.now() });
    startScan().catch((e) => {
      setPhase({ kind: "idle" });
      setMessage({ status: "danger", text: String(e) });
    });
  }

  function toggle(path: string) {
    const next = new Set(selected);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setSelected(next);
  }

  async function trash() {
    const items = confirming;
    setConfirming([]);
    try {
      const r = await moveToTrash(items.map((i) => i.path));
      const gone = new Set(items.map((i) => i.path));
      setPhase((p) =>
        p.kind === "done"
          ? { ...p, result: { ...p.result, top_files: p.result.top_files.filter((i) => !gone.has(i.path)) } }
          : p,
      );
      setSelected(new Set());
      setMessage({
        status: "ok",
        text: `${r.files} ${t("trash.files")} · ${formatBytes(r.bytes, lang)} — ${t("trash.done")}`,
      });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  const feedback = message && <StatusPill status={message.status}>{message.text}</StatusPill>;

  if (phase.kind === "idle") {
    return (
      <Card>
        <p>{t("analyse.intro")}</p>
        <button className="btn btn-primary" onClick={start}>
          {t("analyse.start")}
        </button>
        {feedback}
      </Card>
    );
  }

  if (phase.kind === "scanning") {
    const { progress, start: begun } = phase;
    return (
      <Card>
        <div className="row">
          <StatusPill status="neutral">{t("analyse.running")}</StatusPill>
          <span className="muted">{Math.round((Date.now() - begun) / 1000)} s</span>
        </div>
        <div className="row stats">
          <span>
            <span className="num big">{progress.files.toLocaleString(lang)}</span> {t("analyse.files")}
          </span>
          <span className="num big">{formatBytes(progress.bytes, lang)}</span>
        </div>
        <button className="btn" onClick={() => cancelScan()}>
          {t("common.cancel")}
        </button>
      </Card>
    );
  }

  const r = phase.result;
  const maxCat = Math.max(1, ...r.categories.map((c) => c.bytes));
  const chosen = r.top_files.filter((i) => selected.has(i.path));
  const chosenBytes = chosen.reduce((s, i) => s + i.size, 0);

  return (
    <>
      <Card>
        <div className="row stats">
          <span className="num big">{formatBytes(r.bytes, lang)}</span>
          <span>
            <span className="num">{r.files.toLocaleString(lang)}</span> {t("analyse.files")}
          </span>
          <span className="muted">
            {r.roots.join(" ")} · {(r.duration_ms / 1000).toFixed(1)} s
          </span>
        </div>
        <div className="row">
          {r.cancelled && <StatusPill status="warn">{t("analyse.cancelled")}</StatusPill>}
          {r.denied > 0 && (
            <StatusPill status="warn">
              {r.denied} {t("analyse.denied")}
            </StatusPill>
          )}
          <button className="btn" onClick={start}>
            {t("analyse.restart")}
          </button>
          <button className="btn btn-primary" onClick={() => setReport(true)}>
            {t("report.button")}
          </button>
        </div>
        {r.denied > 0 && !elevated && <AdminBanner elevated={false} />}
        {feedback}
      </Card>

      <Card title={t("analyse.categories")}>
        {[...r.categories]
          .sort((a, b) => b.bytes - a.bytes)
          .map((c) => (
            <SizeBar
              key={c.category}
              label={t(`cat.${c.category}` as Key)}
              value={formatBytes(c.bytes, lang)}
              ratio={c.bytes / maxCat}
            />
          ))}
      </Card>

      <Card title={t("analyse.topFiles")}>
        <p className="muted">{t("analyse.topFilesHint")}</p>
        <ul className="items">
          {r.top_files.slice(0, 50).map((i) => (
            <li key={i.path}>
              <label className="check path" title={i.path}>
                <input type="checkbox" checked={selected.has(i.path)} onChange={() => toggle(i.path)} />
                <span>{i.path}</span>
              </label>
              <span className="num">{formatBytes(i.size, lang)}</span>
              <button className="btn btn-small" onClick={() => revealInExplorer(i.path)}>
                {t("analyse.open")}
              </button>
            </li>
          ))}
        </ul>
        <div className="row">
          <button className="btn btn-primary" disabled={chosen.length === 0} onClick={() => setConfirming(chosen)}>
            {t("trash.button")}
          </button>
          {chosen.length > 0 && (
            <span className="muted">
              {chosen.length} {t("trash.files")} · <span className="num">{formatBytes(chosenBytes, lang)}</span>
            </span>
          )}
        </div>
      </Card>

      <Card title={t("analyse.topDirs")}>
        <ul className="items">
          {r.top_dirs.slice(0, 50).map((i) => (
            <li key={i.path}>
              <span className="path" title={i.path}>
                {i.path}
              </span>
              <span className="num">{formatBytes(i.size, lang)}</span>
              <button className="btn btn-small" onClick={() => revealInExplorer(i.path)}>
                {t("analyse.open")}
              </button>
            </li>
          ))}
        </ul>
      </Card>

      {report && <ReportDialog scan={r} onClose={() => setReport(false)} />}
      <TrashConfirm items={confirming} onConfirm={trash} onCancel={() => setConfirming([])} />
    </>
  );
}
