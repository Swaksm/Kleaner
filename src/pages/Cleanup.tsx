import { useEffect, useRef, useState } from "react";
import { AdminBanner } from "../components/AdminBanner";
import { Card } from "../components/Card";
import { CleanerCard } from "../components/CleanerCard";
import { CleanupPreview } from "../components/CleanupPreview";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { Donut } from "../components/Donut";
import { SizeBar } from "../components/SizeBar";
import { Skeleton } from "../components/Skeleton";
import { StatusPill } from "../components/StatusPill";
import { useT, type Key } from "../i18n/useT";
import {
  createRestorePoint,
  type RestorePoint,
  estimateCleaners,
  listCleaners,
  listDrives,
  type DriveInfo,
  runCleaners,
  type CleanReport,
  type Cleaner,
  type CleanersView,
} from "../lib/api";
import { formatBytes } from "../lib/format";

const GROUPS = ["windows", "browsers", "apps", "dev", "community", "system"] as const;
// Ordre fixe = ordre des couleurs du graphique.
const CHART_GROUPS = ["windows", "browsers", "apps", "dev", "system", "heavy"] as const;

/** `quick` : venu du bouton « Nettoyer sans risque » de l'Accueil, coche les éléments sans risque et ouvre la confirmation. */
export function Cleanup({ quick = false, onQuickDone }: { quick?: boolean; onQuickDone?: () => void }) {
  const { t, lang, tx } = useT();
  const [view, setView] = useState<CleanersView | null>(null);
  const [estimates, setEstimates] = useState<Map<string, CleanReport> | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirm, setConfirm] = useState(false);
  const [preview, setPreview] = useState(false);
  const [heavy, setHeavy] = useState<Cleaner | null>(null);
  const [heavyAck, setHeavyAck] = useState(false);
  // Point de restauration avant un nettoyage lourd : coché par défaut.
  const [restoreFirst, setRestoreFirst] = useState(true);
  const [restoring, setRestoring] = useState(false);
  const [restore, setRestore] = useState<RestorePoint | null>(null);
  // Point impossible : on demande avant de lancer le nettoyage lourd.
  const [noRestore, setNoRestore] = useState<{ id: string; detail: string } | null>(null);
  const [running, setRunning] = useState(false);
  const [results, setResults] = useState<CleanReport[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showEmpty, setShowEmpty] = useState(false);
  // Espace libre des disques avant / après le dernier nettoyage (carte de fin).
  const [drives, setDrives] = useState<{ before: DriveInfo[]; after: DriveInfo[] } | null>(null);
  const doneRef = useRef<HTMLDivElement>(null);

  /// Sans risque et utile : niveau safe, quelque chose à gagner, rien qui bloque.
  const selectable = (c: Cleaner, map: Map<string, CleanReport>) => {
    const r = map.get(c.id);
    // Le catalogue communautaire n'est jamais coché en bloc.
    return c.level === "safe" && c.group !== "community" && !!r && r.bytes > 0 && r.blocked_by.length === 0;
  };

  // Jamais de sélection automatique : l'utilisateur coche lui-même, ou clique « Cocher les éléments sans risque »
  // (ici ou sur l'Accueil). La confirmation montre toujours la liste avant toute suppression.
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

  useEffect(() => {
    if (!quick || !view || !estimates) return;
    const ids = view.cleaners.filter((c) => selectable(c, estimates)).map((c) => c.id);
    setSelected(new Set(ids));
    if (ids.length > 0) setConfirm(true);
    onQuickDone?.();
  }, [quick, view, estimates]);

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
      const fixed = () => listDrives().then((d) => d.filter((x) => x.kind === "fixed"), () => [] as DriveInfo[]);
      const before = await fixed();
      setResults(await runCleaners(ids));
      setDrives({ before, after: await fixed() });
      setSelected(new Set());
      requestAnimationFrame(() => doneRef.current?.scrollIntoView({ behavior: "smooth", block: "start" }));
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }

  async function runHeavy(id: string) {
    setRestore(null);
    if (!restoreFirst) return run([id]);
    setHeavy(null);
    setHeavyAck(false);
    setRestoring(true);
    const point = await createRestorePoint().catch((e): RestorePoint => ({ status: "failed", detail: String(e) }));
    setRestoring(false);
    setRestore(point);
    if (point.status === "failed") setNoRestore({ id, detail: point.detail });
    else await run([id]);
  }

  if (!view) return error ? <StatusPill status="danger">{error}</StatusPill> : <Skeleton lines={6} />;

  const byId = new Map(view.cleaners.map((c) => [c.id, c]));
  const chosen = [...selected].map((id) => byId.get(id)!).filter(Boolean);
  const chosenBytes = chosen.reduce((s, c) => s + (estimates?.get(c.id)?.bytes ?? 0), 0);
  const freed = results?.reduce((s, r) => s + r.bytes, 0) ?? 0;
  const recoverable = view.cleaners.filter((c) => c.group !== "community").reduce((s, c) => s + (estimates?.get(c.id)?.bytes ?? 0), 0);

  return (
    <>
      {results && (
        <div className="card done-card" role="status" ref={doneRef}>
          <div className="done-head">
            <svg className="done-check" viewBox="0 0 24 24" width="40" height="40" aria-hidden fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <path d="M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0z" />
              <path d="m8 12 3 3 5-6" />
            </svg>
            <div>
              <div className="done-big num">{formatBytes(freed, lang)}</div>
              <div className="muted">{t("cleanup.freed")}</div>
            </div>
          </div>
          {drives?.after
            .map((d) => ({ d, b: drives.before.find((x) => x.root === d.root) }))
            .filter(({ d, b }) => b && d.free !== b.free)
            .map(({ d, b }) => (
              <SizeBar
                key={d.root}
                label={`${d.root} · ${formatBytes(b!.free, lang)} → ${formatBytes(d.free, lang)} ${t("cleanup.freeSpace")}`}
                value={`${Math.round(((d.total - d.free) / Math.max(1, d.total)) * 100)} % ${t("analyse.used")}`}
                ratio={(d.total - d.free) / Math.max(1, d.total)}
                usage
              />
            ))}
        </div>
      )}

      <Card>
        <p>{t("cleanup.intro")}</p>
        <AdminBanner elevated={view.elevated} />
        {error && <StatusPill status="danger">{error}</StatusPill>}
      </Card>

      {/* Reste en haut pendant le défilement : on coche plus bas, on nettoie sans remonter. */}
      <div className="cleanup-bar">
        <span className="cleanup-sum">
          {chosen.length === 0 ? (
            <span className="muted">{t("cleanup.nothing")}</span>
          ) : (
            <>
              <strong>{chosen.length}</strong> {t("cleanup.ticked")} · <span className="num">{formatBytes(chosenBytes, lang)}</span>
            </>
          )}
        </span>
        <div className="row">
          <button className={chosen.length === 0 ? "btn btn-primary" : "btn"} disabled={running || !estimates} onClick={selectSafe}>
            {t("cleanup.selectSafe")}
          </button>
          <button className="btn" disabled={chosen.length === 0 || running} onClick={() => setPreview(true)}>
            {t("preview.button")}
          </button>
          <button className={chosen.length > 0 ? "btn btn-primary" : "btn"} disabled={chosen.length === 0 || running} onClick={() => setConfirm(true)}>
            {running ? t("cleanup.running") : chosen.length > 0 ? `${t("cleanup.run")} · ${formatBytes(chosenBytes, lang)}` : t("cleanup.run")}
          </button>
        </div>
        <div className="row cleanup-more">
          <button className="link" disabled={running || chosen.length === 0} onClick={() => setSelected(new Set())}>
            {t("cleanup.selectNone")}
          </button>
          <button className="link" disabled={running || !estimates} onClick={load}>
            {estimates ? t("cleanup.refresh") : t("cleanup.estimating")}
          </button>
          {!estimates && <span className="spinner" role="status" aria-label={t("cleanup.estimating")} />}
        </div>
      </div>

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
                <span>{byId.has(r.id) ? tx(byId.get(r.id)!.label) : r.id}</span>
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
          {restore && restore.status !== "failed" && (
            <StatusPill status="ok">
              {restore.status === "created"
                ? t("cleanup.restoreCreated")
                : `${t("cleanup.restoreRecent")} ${new Date(restore.ts * 1000).toLocaleString(lang)}`}
            </StatusPill>
          )}
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
        const bytes = items.reduce((n, c) => n + (estimates?.get(c.id)?.bytes ?? 0), 0);
        const ticked = items.filter((c) => selected.has(c.id)).length;
        const summary = [estimates && formatBytes(bytes, lang), ticked > 0 && `${ticked} ${t("cleanup.ticked")}`].filter(Boolean).join(" · ");
        return (
          <Card key={g} title={t(`group.${g}` as Key)} fold={{ id: `cleanup-${g}`, summary }}>
            {g === "community" && <p className="muted">{t("cleanup.communityIntro")}</p>}
            {items.length === 0 && <p className="muted">{t("cleanup.nothing")}</p>}
            {items.map((c) => (
              <CleanerCard
                key={c.id}
                cleaner={c}
                report={estimates?.get(c.id)}
                locked={lockReason(c)}
                checked={selected.has(c.id)}
                onToggle={() => toggle(c.id)}
                compact={g === "community"}
              />
            ))}
          </Card>
        );
      })}

      <Card title={t("group.heavy")} fold={{ id: "cleanup-heavy" }}>
        <p className="muted">{t("cleanup.heavyIntro")}</p>
        {restoring && (
          <span className="loading" role="status">
            <span className="spinner" aria-hidden /> {t("cleanup.restoring")}
          </span>
        )}
        {view.cleaners
          .filter((c) => c.level === "heavy")
          .map((c) => (
            <CleanerCard key={c.id} cleaner={c} report={estimates?.get(c.id)} locked={lockReason(c)}>
              <button className="btn btn-danger" disabled={!!lockReason(c) || running || restoring} onClick={() => setHeavy(c)}>
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
              {tx(c.label)} — {tx(c.effect)}
            </li>
          ))}
        </ul>
      </ConfirmDialog>

      <ConfirmDialog
        open={heavy !== null}
        title={heavy ? tx(heavy.label) : ""}
        confirmLabel={t("cleanup.heavyRun")}
        cancelLabel={t("common.cancel")}
        danger
        confirmDisabled={!heavyAck}
        onConfirm={() => heavy && runHeavy(heavy.id)}
        onCancel={() => {
          setHeavy(null);
          setHeavyAck(false);
        }}
      >
        {heavy && (
          <>
            <p>{tx(heavy.what)}</p>
            {heavy.command && <code>{heavy.command}</code>}
            <p className="danger">{tx(heavy.effect)}</p>
            <label className="check">
              <input type="checkbox" checked={heavyAck} onChange={(e) => setHeavyAck(e.target.checked)} />
              {t("cleanup.heavyAck")}
            </label>
            {!heavyAck && <p className="muted">{t("cleanup.heavyAckHint")}</p>}
            <label className="check">
              <input type="checkbox" checked={restoreFirst} onChange={(e) => setRestoreFirst(e.target.checked)} />
              {t("cleanup.restoreFirst")}
            </label>
            <p className="muted small">{t("cleanup.restoreHint")}</p>
          </>
        )}
      </ConfirmDialog>

      <ConfirmDialog
        open={noRestore !== null}
        title={t("cleanup.noRestoreTitle")}
        confirmLabel={t("cleanup.noRestoreContinue")}
        cancelLabel={t("common.cancel")}
        danger
        onConfirm={() => {
          const id = noRestore!.id;
          setNoRestore(null);
          run([id]);
        }}
        onCancel={() => setNoRestore(null)}
      >
        <p>{t("cleanup.noRestore")}</p>
        {noRestore && <code>{noRestore.detail.split("\n").filter(Boolean).pop()}</code>}
      </ConfirmDialog>
    </>
  );
}
