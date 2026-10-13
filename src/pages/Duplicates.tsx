import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { StatusPill } from "../components/StatusPill";
import { TrashConfirm } from "../components/TrashConfirm";
import { useT } from "../i18n/useT";
import {
  cancelDuplicates,
  findDuplicates,
  moveToTrash,
  onDupProgress,
  revealInExplorer,
  type DupGroup,
  type Item,
} from "../lib/api";
import { formatBytes, formatDay } from "../lib/format";
import { useScanResult } from "../lib/scanStore";

export function Duplicates() {
  const { t, lang } = useT();
  const scan = useScanResult();
  const [groups, setGroups] = useState<DupGroup[] | null>(null);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState<Item[]>([]);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  useEffect(() => {
    const un = onDupProgress(setProgress);
    un.catch(() => {});
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  }, []);

  // Une nouvelle analyse rend la liste précédente caduque.
  useEffect(() => {
    setGroups(null);
    setSelected(new Set());
  }, [scan?.duration_ms]);

  async function search() {
    setRunning(true);
    setProgress(null);
    setMessage(null);
    try {
      setGroups(await findDuplicates());
      setSelected(new Set());
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    } finally {
      setRunning(false);
    }
  }

  /** Une copie reste toujours : on ne peut pas cocher le dernier fichier non coché d'un groupe. */
  function toggle(g: DupGroup, path: string) {
    const next = new Set(selected);
    if (next.has(path)) next.delete(path);
    else if (g.files.filter((f) => !next.has(f.path)).length > 1) next.add(path);
    setSelected(next);
  }

  /** Coche tout sauf la copie la plus ancienne de chaque groupe. */
  function keepOldest() {
    setSelected(new Set(groups?.flatMap((g) => g.files.slice(1).map((f) => f.path)) ?? []));
  }

  async function trash() {
    const items = confirming;
    setConfirming([]);
    try {
      const r = await moveToTrash(items.map((i) => i.path));
      const gone = new Set(items.map((i) => i.path));
      setGroups(
        (prev) =>
          prev
            ?.map((g) => ({ ...g, files: g.files.filter((f) => !gone.has(f.path)) }))
            .filter((g) => g.files.length > 1)
            .map((g) => ({ ...g, wasted: g.size * (g.files.length - 1) })) ?? null,
      );
      setSelected(new Set());
      setMessage({ status: "ok", text: `${r.files} ${t("trash.files")} · ${formatBytes(r.bytes, lang)} — ${t("trash.done")}` });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  if (!scan) {
    return (
      <Card>
        <p>{t("media.needScan")}</p>
      </Card>
    );
  }

  const all = groups?.flatMap((g) => g.files) ?? [];
  const chosen = all.filter((f) => selected.has(f.path));
  const wasted = groups?.reduce((s, g) => s + g.wasted, 0) ?? 0;

  return (
    <>
      <Card>
        <p>{t("dups.intro")}</p>
        <div className="row">
          <button className="btn btn-primary" disabled={running} onClick={search}>
            {groups ? t("dups.again") : t("dups.search")}
          </button>
          {running && (
            <>
              <span className="loading" role="status">
                <span className="spinner" aria-hidden /> {t("dups.comparing")}
                {progress && progress.total > 0 && ` ${Math.round((progress.done / progress.total) * 100)} %`}
              </span>
              <button className="btn" onClick={() => cancelDuplicates()}>
                {t("common.cancel")}
              </button>
            </>
          )}
        </div>
        {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      </Card>

      {groups && (
        <Card>
          <div className="row stats">
            <span className="num big">{formatBytes(wasted, lang)}</span>
            <span className="muted">
              {groups.length} {t("dups.groups")}
            </span>
          </div>
          {groups.length === 0 ? (
            <p>{t("dups.none")}</p>
          ) : (
            <>
              <div className="row">
                <button className="btn" onClick={keepOldest}>
                  {t("dups.keepOldest")}
                </button>
                <button className="btn" disabled={selected.size === 0} onClick={() => setSelected(new Set())}>
                  {t("cleanup.selectNone")}
                </button>
                <button className="btn btn-primary" disabled={chosen.length === 0} onClick={() => setConfirming(chosen)}>
                  {t("trash.button")}
                </button>
                <span className="muted">
                  {chosen.length} · <span className="num">{formatBytes(chosen.reduce((s, f) => s + f.size, 0), lang)}</span>
                </span>
              </div>
              <div className="dup-groups">
                {groups.map((g) => (
                  <div key={g.files[0].path} className="dup-group">
                    <div className="row">
                      <strong className="num">{formatBytes(g.size, lang)}</strong>
                      <span className="muted">
                        × {g.files.length} · {formatBytes(g.wasted, lang)} {t("dups.recoverable")}
                      </span>
                    </div>
                    <ul className="items grid-dup">
                      {g.files.map((f) => {
                        const lastLeft = !selected.has(f.path) && g.files.filter((x) => !selected.has(x.path)).length === 1;
                        return (
                          <li key={f.path}>
                            <label className="check path" title={lastLeft ? t("dups.lastCopy") : f.path}>
                              <input type="checkbox" checked={selected.has(f.path)} disabled={lastLeft} onChange={() => toggle(g, f.path)} />
                              <span>{f.path}</span>
                            </label>
                            <span className="muted small">{formatDay(f.modified, lang)}</span>
                            <button className="btn btn-small" onClick={() => revealInExplorer(f.path)}>
                              {t("analyse.open")}
                            </button>
                          </li>
                        );
                      })}
                    </ul>
                  </div>
                ))}
              </div>
            </>
          )}
        </Card>
      )}

      <TrashConfirm items={confirming} onConfirm={trash} onCancel={() => setConfirming([])} />
    </>
  );
}
