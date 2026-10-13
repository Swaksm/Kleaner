import { useState } from "react";
import { Card } from "../components/Card";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { SortHeader } from "../components/SortHeader";
import { StatusPill } from "../components/StatusPill";
import { useT, type Key } from "../i18n/useT";
import { deleteArtifacts, revealInExplorer, type Artifact } from "../lib/api";
import { formatBytes, formatDay } from "../lib/format";
import { forgetPaths, useScanResult } from "../lib/scanStore";
import { sortBy, type SortState } from "../lib/sort";

type SortKey = "size" | "project" | "kind" | "date";
const MONTHS = [0, 1, 3, 6, 12];
const DAY = 86_400;

/** Commande qui recrée le dossier, affichée avant suppression. */
const REBUILD: Record<Artifact["kind"], string> = {
  node: "npm install",
  next: "npm run build",
  python: "python -m venv .venv && pip install -r requirements.txt",
  rust: "cargo build",
  gradle: "gradle build",
  build: "gradle build / flutter build",
  dart: "flutter pub get",
  dotnet: "dotnet build",
};

export function Projects() {
  const { t, lang } = useT();
  const scan = useScanResult();
  const [months, setMonths] = useState(3);
  const [sort, setSort] = useState<SortState<SortKey>>({ key: "size", dir: "desc" });
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirm, setConfirm] = useState(false);
  const [running, setRunning] = useState(false);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  if (!scan) {
    return (
      <Card>
        <p>{t("media.needScan")}</p>
      </Card>
    );
  }

  const cutoff = Date.now() / 1000 - months * 30 * DAY;
  const shown = sortBy(
    scan.artifacts.filter((a) => months === 0 || a.project_modified < cutoff),
    sort,
    { size: (a) => a.size, project: (a) => a.project, kind: (a) => a.kind, date: (a) => a.project_modified },
  );
  const chosen = scan.artifacts.filter((a) => selected.has(a.path));
  const sum = (l: Artifact[]) => l.reduce((s, a) => s + a.size, 0);

  function toggle(path: string) {
    const next = new Set(selected);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setSelected(next);
  }

  async function remove() {
    setConfirm(false);
    setRunning(true);
    try {
      const r = await deleteArtifacts(chosen.map((a) => a.path));
      forgetPaths(chosen.map((a) => a.path));
      setSelected(new Set());
      setMessage({
        status: r.skipped > 0 ? "danger" : "ok",
        text: `${formatBytes(r.bytes, lang)} ${t("cleanup.freed")}${r.skipped > 0 ? ` · ${r.skipped} ${t("journal.skipped")}` : ""}`,
      });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    } finally {
      setRunning(false);
    }
  }

  return (
    <>
      <Card>
        <p>{t("projects.intro")}</p>
        <div className="row">
          <label>
            {t("projects.untouched")}{" "}
            <select value={months} onChange={(e) => setMonths(+e.target.value)}>
              {MONTHS.map((m) => (
                <option key={m} value={m}>
                  {m === 0 ? t("projects.any") : `${m} ${t("projects.months")}`}
                </option>
              ))}
            </select>
          </label>
          <span className="num big">{formatBytes(sum(shown), lang)}</span>
          <span className="muted">
            {shown.length} {t("projects.folders")}
          </span>
        </div>
        <div className="row">
          <button className="btn" onClick={() => setSelected(new Set(shown.map((a) => a.path)))}>
            {t("media.selectVisible")}
          </button>
          <button className="btn" disabled={selected.size === 0} onClick={() => setSelected(new Set())}>
            {t("cleanup.selectNone")}
          </button>
          <button className="btn btn-danger" disabled={chosen.length === 0 || running} onClick={() => setConfirm(true)}>
            {running ? t("cleanup.running") : t("projects.delete")}
          </button>
          <span className="muted">
            {chosen.length} · <span className="num">{formatBytes(sum(chosen), lang)}</span>
          </span>
        </div>
        {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      </Card>

      <Card>
        {shown.length === 0 ? (
          <p className="muted">{t("projects.none")}</p>
        ) : (
          <ul className="items grid-projects">
            <SortHeader
              cells={[
                { sorts: [["project", t("projects.project"), false]] },
                { sorts: [["kind", t("projects.kind"), false]] },
                { sorts: [["date", t("projects.lastChange"), true]] },
                { sorts: [["size", t("sort.size"), true]], end: true },
                null,
              ]}
              value={sort}
              onChange={setSort}
            />
            {shown.map((a) => (
              <li key={a.path}>
                <label className="check path" title={a.path}>
                  <input type="checkbox" checked={selected.has(a.path)} onChange={() => toggle(a.path)} />
                  <span>{a.path}</span>
                </label>
                <span className="muted small">{t(`projects.kind.${a.kind}` as Key)}</span>
                <span className="muted small">{formatDay(a.project_modified, lang)}</span>
                <span className="num end">{formatBytes(a.size, lang)}</span>
                <button className="btn btn-small" onClick={() => revealInExplorer(a.project)}>
                  {t("analyse.open")}
                </button>
              </li>
            ))}
          </ul>
        )}
      </Card>

      <ConfirmDialog
        open={confirm}
        title={t("projects.confirmTitle")}
        confirmLabel={t("projects.delete")}
        cancelLabel={t("common.cancel")}
        danger
        onConfirm={remove}
        onCancel={() => setConfirm(false)}
      >
        <p>
          {chosen.length} · <span className="num">{formatBytes(sum(chosen), lang)}</span>
        </p>
        <p className="danger">{t("projects.permanent")}</p>
        <ul className="paths">
          {chosen.map((a) => (
            <li key={a.path}>
              {a.path}
              <br />
              <span className="muted">→ {REBUILD[a.kind]}</span>
            </li>
          ))}
        </ul>
      </ConfirmDialog>
    </>
  );
}
