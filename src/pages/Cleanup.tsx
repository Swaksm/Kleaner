import { useEffect, useState } from "react";
import { AdminBanner } from "../components/AdminBanner";
import { Card } from "../components/Card";
import { CleanerCard } from "../components/CleanerCard";
import { CleanupPreview } from "../components/CleanupPreview";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { Donut } from "../components/Donut";
import { StatusPill } from "../components/StatusPill";
import { useT, type Key } from "../i18n/useT";
import {
  estimateCleaners,
  listCleaners,
  runCleaners,
  type CleanReport,
  type Cleaner,
  type CleanersView,
} from "../lib/api";
import { formatBytes } from "../lib/format";

const GROUPS = ["windows", "browsers", "apps", "dev", "system"] as const;
// Ordre fixe = ordre des couleurs du graphique.
const CHART_GROUPS = ["windows", "browsers", "apps", "dev", "system", "heavy"] as const;

export function Cleanup() {
  const { t, lang, tl } = useT();
  const [view, setView] = useState<CleanersView | null>(null);
  const [estimates, setEstimates] = useState<Map<string, CleanReport> | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirm, setConfirm] = useState(false);
  const [preview, setPreview] = useState(false);
  const [heavy, setHeavy] = useState<Cleaner | null>(null);
  const [heavyAck, setHeavyAck] = useState(false);
  const [running, setRunning] = useState(false);
  const [results, setResults] = useState<CleanReport[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showEmpty, setShowEmpty] = useState(false);

  /// Sans risque et utile : niveau safe, quelque chose à gagner, rien qui bloque.
  const selectable = (c: Cleaner, map: Map<string, CleanReport>) => {
    const r = map.get(c.id);
    return c.level === "safe" && !!r && r.bytes > 0 && r.blocked_by.length === 0;
  };

  // Jamais de sélection automatique : l'utilisateur coche lui-même, ou clique « Cocher les éléments sans risque ».
  async function load() {
    setEstimates(null);
    try {
      const v = await listCleaners();
      setView(v);
      const list = await estimateCleaners();
      const map = new Map(list.map((r) => [r.id, r]));
      setEstimates(map);
      // On garde la sélection de l'utilisateur, sauf ce qui est devenu vide ou bloqué.
      setSelected((prev) => new Set([...prev].filter((id) => v.cleaners.some((c) => c.id === id && selectable(c, map)))));
    } catch (e) {
      setError(String(e));
    }
  }

  function selectSafe() {
    if (!view || !estimates) return;
    setSelected(new Set(view.cleaners.filter((c) => selectable(c, estimates)).map((c) => c.id)));
  }

  useEffect(() => {
    load();
  }, []);

  function lockReason(c: Cleaner): string | undefined {
    if (c.admin && !view?.elevated) return t("cleanup.needsAdmin");
    const r = estimates?.get(c.id);
    if (r && r.blocked_by.length > 0) return `${t("cleanup.closeFirst")} ${r.blocked_by.join(", ")}`;
    return undefined;
  }

  function toggle(id: string) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelected(next);
  }

  async function run(ids: string[]) {
    setConfirm(false);
    setHeavy(null);
    setHeavyAck(false);
    setRunning(true);
    setError(null);
    try {
      setResults(await runCleaners(ids));
      setSelected(new Set());
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }

  if (!view) return error ? <StatusPill status="danger">{error}</StatusPill> : <p className="muted">…</p>;

  const byId = new Map(view.cleaners.map((c) => [c.id, c]));
  const chosen = [...selected].map((id) => byId.get(id)!).filter(Boolean);
  const chosenBytes = chosen.reduce((s, c) => s + (estimates?.get(c.id)?.bytes ?? 0), 0);
  const freed = results?.reduce((s, r) => s + r.bytes, 0) ?? 0;
  const recoverable = [...(estimates?.values() ?? [])].reduce((s, r) => s + r.bytes, 0);

  return (
    <>
      <Card>
        <p>{t("cleanup.intro")}</p>
        <AdminBanner elevated={view.elevated} />
        <div className="row">
          <button className="btn btn-primary" disabled={chosen.length === 0 || running} onClick={() => setConfirm(true)}>
            {running ? t("cleanup.running") : t("cleanup.run")}
          </button>
          <button className="btn" disabled={chosen.length === 0 || running} onClick={() => setPreview(true)}>
            {t("preview.button")}
          </button>
          <span className="muted">
            {chosen.length} · <span className="num">{formatBytes(chosenBytes, lang)}</span>
          </span>
          <button className="btn" disabled={running || !estimates} onClick={selectSafe}>
            {t("cleanup.selectSafe")}
          </button>
          <button className="btn" disabled={running || chosen.length === 0} onClick={() => setSelected(new Set())}>
            {t("cleanup.selectNone")}
          </button>
          <button className="btn" disabled={running || !estimates} onClick={load}>
            {estimates ? t("cleanup.refresh") : t("cleanup.estimating")}
          </button>
          {!estimates && (
            <span className="loading" role="status">
              <span className="spinner" aria-hidden /> {t("cleanup.estimating")}
            </span>
          )}
        </div>
        {error && <StatusPill status="danger">{error}</StatusPill>}
      </Card>

      {estimates && (
        <Card title={t("cleanup.chart")}>
          <Donut
            centerLabel={t("cleanup.recoverable")}
            centerValue={formatBytes(recoverable, lang)}
            slices={CHART_GROUPS.map((g, i) => {
              const bytes = view.cleaners
                .filter((c) => c.group === g)
                .reduce((s, c) => s + (estimates.get(c.id)?.bytes ?? 0), 0);
              return { key: g, label: t(`group.${g}` as Key), value: bytes, display: formatBytes(bytes, lang), slot: i + 1 };
            }).sort((a, b) => b.value - a.value)}
          />
          {!view.elevated && <p className="muted">{t("cleanup.chartAdminHint")}</p>}
        </Card>
      )}

      {results && (
        <Card title={t("cleanup.results")}>
          <p>
            <span className="num big">{formatBytes(freed, lang)}</span> {t("cleanup.freed")}
          </p>
          <ul className="items">
            {results.map((r) => (
              <li key={r.id}>
                <span>{byId.get(r.id)?.label[tl] ?? r.id}</span>
                <span className="num">{formatBytes(r.bytes, lang)}</span>
                <span>
                  {r.error ? (
                    <StatusPill status="danger">{r.error}</StatusPill>
                  ) : r.skipped > 0 ? (
                    <StatusPill status="warn">
                      {r.skipped} {t("journal.skipped")}
                    </StatusPill>
                  ) : (
                    <StatusPill status="ok">{t("journal.done")}</StatusPill>
                  )}
                </span>
              </li>
            ))}
          </ul>
          {results.some((r) => r.skipped > 0) && <p className="muted">{t("cleanup.skippedWhy")}</p>}
        </Card>
      )}

      {estimates && (
        <label className="check">
          <input type="checkbox" checked={showEmpty} onChange={(e) => setShowEmpty(e.target.checked)} />
          {t("cleanup.showEmpty")}
        </label>
      )}

      {GROUPS.map((g) => {
        const all = view.cleaners.filter((c) => c.group === g && c.level === "safe");
        // Logiciel absent ou cache déjà vide : masqué, sauf si l'utilisateur veut tout voir.
        // Les éléments admin non estimés restent visibles (on ne sait pas encore s'ils sont vides).
        const empty = (c: Cleaner) => !!estimates && !(c.admin && !view.elevated) && (estimates.get(c.id)?.bytes ?? 0) === 0;
        const items = showEmpty ? all : all.filter((c) => !empty(c));
        if (all.length === 0) return null;
        return (
          <Card key={g} title={t(`group.${g}` as Key)}>
            {items.length === 0 && <p className="muted">{t("cleanup.nothing")}</p>}
            {items.map((c) => (
              <CleanerCard
                key={c.id}
                cleaner={c}
                report={estimates?.get(c.id)}
                locked={lockReason(c)}
                checked={selected.has(c.id)}
                onToggle={() => toggle(c.id)}
              />
            ))}
          </Card>
        );
      })}

      <Card title={t("group.heavy")}>
        <p className="muted">{t("cleanup.heavyIntro")}</p>
        {view.cleaners
          .filter((c) => c.level === "heavy")
          .map((c) => (
            <CleanerCard key={c.id} cleaner={c} report={estimates?.get(c.id)} locked={lockReason(c)}>
              <button className="btn btn-danger" disabled={!!lockReason(c) || running} onClick={() => setHeavy(c)}>
                {t("cleanup.heavyRun")}
              </button>
            </CleanerCard>
          ))}
      </Card>

      {preview && <CleanupPreview cleaners={chosen} onClose={() => setPreview(false)} />}

      <ConfirmDialog
        open={confirm}
        title={t("cleanup.confirmTitle")}
        confirmLabel={t("cleanup.run")}
        cancelLabel={t("common.cancel")}
        danger
        onConfirm={() => run(chosen.map((c) => c.id))}
        onCancel={() => setConfirm(false)}
      >
        <p>
          {chosen.length} · <span className="num">{formatBytes(chosenBytes, lang)}</span>
        </p>
        <p className="danger">{t("cleanup.permanent")}</p>
        <button
          className="btn"
          onClick={() => {
            setConfirm(false);
            setPreview(true);
          }}
        >
          {t("preview.button")}
        </button>
        <ul className="paths">
          {chosen.map((c) => (
            <li key={c.id}>
              {c.label[tl]} — {c.effect[tl]}
            </li>
          ))}
        </ul>
      </ConfirmDialog>

      <ConfirmDialog
        open={heavy !== null}
        title={heavy ? heavy.label[tl] : ""}
        confirmLabel={t("cleanup.heavyRun")}
        cancelLabel={t("common.cancel")}
        danger
        confirmDisabled={!heavyAck}
        onConfirm={() => heavy && run([heavy.id])}
        onCancel={() => {
          setHeavy(null);
          setHeavyAck(false);
        }}
      >
        {heavy && (
          <>
            <p>{heavy.what[tl]}</p>
            {heavy.command && <code>{heavy.command}</code>}
            <p className="danger">{heavy.effect[tl]}</p>
            <label className="check">
              <input type="checkbox" checked={heavyAck} onChange={(e) => setHeavyAck(e.target.checked)} />
              {t("cleanup.heavyAck")}
            </label>
            {!heavyAck && <p className="muted">{t("cleanup.heavyAckHint")}</p>}
          </>
        )}
      </ConfirmDialog>
    </>
  );
}
