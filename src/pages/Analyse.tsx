import { useEffect, useState } from "react";
import { AdminBanner } from "../components/AdminBanner";
import { Card } from "../components/Card";
import { DiskMap } from "../components/DiskMap";
import { Donut } from "../components/Donut";
import { DrivePicker } from "../components/DrivePicker";
import { SizeBar } from "../components/SizeBar";
import { StatusPill } from "../components/StatusPill";
import { ReportDialog } from "../components/ReportDialog";
import { SortHeader, type HeaderCell } from "../components/SortHeader";
import { TrashConfirm } from "../components/TrashConfirm";
import { ExplainDialog, VerdictPill } from "../components/Verdict";
import { useT, type Key } from "../i18n/useT";
import {
  cancelScan,
  explainPaths,
  listDrives,
  listCleaners,
  moveToTrash,
  onScanDone,
  onScanProgress,
  revealInExplorer,
  startScan,
  type Category,
  type DriveInfo,
  type Explanation,
  type Verdict,
  type Item,
  type ScanProgress,
} from "../lib/api";
import { formatBytes, formatDay } from "../lib/format";
import { forgetPaths, setScanResult, useScanResult } from "../lib/scanStore";
import { useRecoverable } from "../lib/useRecoverable";
import { baseName, sortBy, type SortState } from "../lib/sort";

// Ordre fixe des catégories = ordre des couleurs du graphique.
const CATEGORY_ORDER: Category[] = ["videos", "images", "audio", "archives", "installers", "other"];

type Phase =
  | { kind: "idle" }
  | { kind: "scanning"; progress: ScanProgress; start: number }
  | { kind: "done" };

type ItemKey = "size" | "name" | "date" | "verdict";
// Tri par verdict : du plus dangereux au plus sûr.
const VERDICT_ORDER: Verdict[] = ["system", "breaks", "uninstall", "personal", "unknown", "cleanup", "safe"];

type Message = { status: "ok" | "danger"; text: string };

export function Analyse({ onGoCleanup, scanRequest }: { onGoCleanup: () => void; scanRequest: number }) {
  const { t, lang } = useT();
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState<Item[]>([]);
  const [message, setMessage] = useState<Message | null>(null);
  const [elevated, setElevated] = useState(true);
  const [report, setReport] = useState(false);
  const [showAll, setShowAll] = useState(false);
  const [fileSort, setFileSort] = useState<SortState<ItemKey>>({ key: "size", dir: "desc" });
  const [dirSort, setDirSort] = useState<SortState<ItemKey>>({ key: "size", dir: "desc" });
  const scan = useScanResult();
  const recoverable = useRecoverable(scan);
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [roots, setRoots] = useState<Set<string>>(new Set());
  // Demande d'analyse venue de l'Accueil (une fois les disques connus).
  useEffect(() => {
    if (scanRequest > 0 && drives.length > 0) start();
  }, [scanRequest, drives.length > 0]);
  const [explanations, setExplanations] = useState<Map<string, Explanation>>(new Map());
  const [explaining, setExplaining] = useState<Explanation | null>(null);

  // Verdict « est-ce que ça casse ? » de chaque gros fichier et dossier, calculé une fois par analyse.
  useEffect(() => {
    if (!scan) return;
    const paths = [...scan.top_files, ...scan.top_dirs].map((i) => i.path);
    explainPaths(paths).then((list) => setExplanations(new Map(list.map((e) => [e.path, e]))), () => {});
  }, [scan?.duration_ms, scan?.roots.join("|")]);

  useEffect(() => {
    listCleaners().then((v) => setElevated(v.elevated), () => {});
    // Disques internes cochés par défaut ; clés USB et lecteurs réseau à cocher soi-même.
    listDrives().then((d) => {
      setDrives(d);
      setRoots(new Set(d.filter((x) => x.kind === "fixed").map((x) => x.root)));
    }, () => {});
    const unlisten = [
      onScanProgress((progress) => setPhase((p) => (p.kind === "scanning" ? { ...p, progress } : p))),
      onScanDone((result) => {
        setScanResult(result);
        setPhase({ kind: "done" });
        setSelected(new Set());
      }),
    ];
    unlisten.forEach((u) => u.catch(() => {}));
    return () => unlisten.forEach((u) => u.then((f) => f()).catch(() => {}));
  }, []);

  function start() {
    setMessage(null);
    setPhase({ kind: "scanning", progress: { files: 0, bytes: 0, denied: 0 }, start: Date.now() });
    startScan(drives.length > 0 ? [...roots] : undefined).catch((e) => {
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
      forgetPaths(items.map((i) => i.path));
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
        <DrivePicker drives={drives} selected={roots} onChange={setRoots} />
        <button className="btn btn-primary" disabled={drives.length > 0 && roots.size === 0} onClick={start}>
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

  const r = scan;
  if (!r) return null;
  const itemGetters: Record<ItemKey, (i: Item) => string | number> = {
    size: (i) => i.size,
    name: (i) => baseName(i.path),
    date: (i) => i.modified,
    verdict: (i) => VERDICT_ORDER.indexOf(explanations.get(i.path)?.verdict ?? "unknown"),
  };
  const files = sortBy(r.top_files, fileSort, itemGetters);
  // Système : la garde refuserait de toute façon, on ne propose pas la case.
  const locked = (path: string) => explanations.get(path)?.verdict === "system";
  const dirs = sortBy(r.top_dirs, dirSort, itemGetters);
  const chosen = r.top_files.filter((i) => selected.has(i.path) && !locked(i.path));
  const free = r.drives.reduce((sum, d) => sum + d.free, 0);
  const chosenBytes = chosen.reduce((s, i) => s + i.size, 0);
  const headerCells = (first: string): HeaderCell<ItemKey>[] => [
    { sorts: [["name", first, false]] },
    { sorts: [["verdict", t("verdict.col"), false]] },
    { sorts: [["date", t("sort.date"), true]] },
    { sorts: [["size", t("sort.size"), true]], end: true },
    null,
  ];
  const showAllButton = (r.top_files.length > 50 || r.top_dirs.length > 50) && (
    <button className="btn" onClick={() => setShowAll(!showAll)}>
      {showAll ? t("analyse.showLess") : t("analyse.showAll")}
    </button>
  );

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
        <details className="drives-again">
          <summary className="muted">{t("drives.change")}</summary>
          <DrivePicker drives={drives} selected={roots} onChange={setRoots} />
        </details>
        {r.drives.map((d) => (
          <SizeBar
            key={d.root}
            label={`${d.root} · ${formatBytes(d.free, lang)} ${t("analyse.freeOf")} ${formatBytes(d.total, lang)}`}
            value={`${Math.round(((d.total - d.free) / Math.max(1, d.total)) * 100)} % ${t("analyse.used")}`}
            ratio={(d.total - d.free) / Math.max(1, d.total)}
          />
        ))}
        <div className="row">
          {r.full_access && <StatusPill status="ok">{t("analyse.fullAccess")}</StatusPill>}
          {r.cancelled && <StatusPill status="warn">{t("analyse.cancelled")}</StatusPill>}
          {r.denied > 0 && (
            <StatusPill status="warn">
              {r.denied} {t("analyse.denied")}
            </StatusPill>
          )}
          <button className="btn" disabled={drives.length > 0 && roots.size === 0} onClick={start}>
            {t("analyse.restart")}
          </button>
          <button className="btn btn-primary" onClick={() => setReport(true)}>
            {t("report.button")}
          </button>
        </div>
        {r.denied > 0 && !elevated && <AdminBanner elevated={false} />}
        {feedback}
      </Card>

      {recoverable !== null && recoverable > 0 && (
        <div className="card cta">
          <div>
            <strong>
              {t("cta.free")} <span className="num">{formatBytes(recoverable, lang)}</span> {t("cta.safely")}
            </strong>
            <p className="muted small">{t("cta.detail")}</p>
          </div>
          <button className="btn btn-primary" onClick={onGoCleanup}>
            {t("cta.go")} →
          </button>
        </div>
      )}

      <Card title={t("analyse.categories")}>
        <Donut
          centerLabel={t("analyse.total")}
          centerValue={formatBytes(r.bytes + free, lang)}
          slices={[
            ...[...r.categories]
              .sort((a, b) => b.bytes - a.bytes)
              .map((c) => ({
                key: c.category,
                label: t(`cat.${c.category}` as Key),
                value: c.bytes,
                display: formatBytes(c.bytes, lang),
                // Couleur fixée par catégorie, pas par rang.
                slot: CATEGORY_ORDER.indexOf(c.category) + 1,
              })),
            // Espace libre en dernier, en gris neutre.
            { key: "free", label: t("analyse.free"), value: free, display: formatBytes(free, lang), slot: 7 },
          ]}
        />
      </Card>

      <Card title={t("map.title")}>
        <DiskMap scan={r} />
      </Card>

      <Card title={t("analyse.topDirsChart")}>
        {r.top_dirs.slice(0, 10).map((d) => (
          <SizeBar
            key={d.path}
            label={d.path}
            value={formatBytes(d.size, lang)}
            ratio={d.size / Math.max(1, r.top_dirs[0]?.size ?? 1)}
          />
        ))}
      </Card>

      <Card title={t("analyse.topFiles")}>
        <p className="muted">{t("analyse.topFilesHint")}</p>
        <ul className="items grid-5">
          <SortHeader cells={headerCells(t("sort.name"))} value={fileSort} onChange={setFileSort} />
          {files.slice(0, showAll ? undefined : 50).map((i) => (
            <li key={i.path}>
              <label className="check path" title={locked(i.path) ? t("verdict.system") : i.path}>
                <input
                  type="checkbox"
                  checked={selected.has(i.path)}
                  disabled={locked(i.path)}
                  onChange={() => toggle(i.path)}
                />
                <span>{i.path}</span>
              </label>
              <VerdictPill e={explanations.get(i.path)} onOpen={setExplaining} />
              <span className="muted small">{formatDay(i.modified, lang)}</span>
              <span className="num end">{formatBytes(i.size, lang)}</span>
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
          {showAllButton}
        </div>
      </Card>

      <Card title={t("analyse.topDirs")}>
        <ul className="items grid-5">
          <SortHeader cells={headerCells(t("sort.folder"))} value={dirSort} onChange={setDirSort} />
          {dirs.slice(0, showAll ? undefined : 50).map((i) => (
            <li key={i.path}>
              <span className="path" title={i.path}>
                {i.path}
              </span>
              <VerdictPill e={explanations.get(i.path)} onOpen={setExplaining} />
              <span className="muted small">{formatDay(i.modified, lang)}</span>
              <span className="num end">{formatBytes(i.size, lang)}</span>
              <button className="btn btn-small" onClick={() => revealInExplorer(i.path)}>
                {t("analyse.open")}
              </button>
            </li>
          ))}
        </ul>
        {showAllButton}
      </Card>

      {report && <ReportDialog scan={r} onClose={() => setReport(false)} />}
      {explaining && <ExplainDialog e={explaining} onClose={() => setExplaining(null)} />}
      <TrashConfirm items={confirming} explanations={explanations} onConfirm={trash} onCancel={() => setConfirming([])} />
    </>
  );
}
