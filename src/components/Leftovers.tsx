import { useState } from "react";
import { useT, type Key } from "../i18n/useT";
import { findLeftovers, revealInExplorer, trashLeftovers, type Leftover } from "../lib/api";
import { formatBytes, formatDay } from "../lib/format";
import { Card } from "./Card";
import { StatusPill } from "./StatusPill";
import { TrashConfirm } from "./TrashConfirm";

/** Restes de programmes désinstallés : dossiers d'AppData (et autres) qui n'appartiennent plus à rien. */
export function Leftovers() {
  const { t, lang } = useT();
  const [list, setList] = useState<Leftover[] | null>(null);
  const [running, setRunning] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState<Leftover[]>([]);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  async function search() {
    setRunning(true);
    setMessage(null);
    try {
      setList(await findLeftovers());
      setSelected(new Set());
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    } finally {
      setRunning(false);
    }
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
      const r = await trashLeftovers(items.map((i) => i.path));
      const gone = new Set(items.map((i) => i.path));
      setList((prev) => prev?.filter((l) => !gone.has(l.path)) ?? null);
      setSelected(new Set());
      setMessage({ status: "ok", text: `${r.files} · ${formatBytes(r.bytes, lang)} — ${t("trash.done")}` });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  const chosen = list?.filter((l) => selected.has(l.path)) ?? [];
  const total = (l: Leftover[]) => l.reduce((s, x) => s + x.size, 0);

  return (
    <Card title={t("leftovers.title")}>
      <p>{t("leftovers.intro")}</p>
      <div className="row">
        <button className="btn btn-primary" disabled={running} onClick={search}>
          {list ? t("dups.again") : t("leftovers.search")}
        </button>
        {running && (
          <span className="loading" role="status">
            <span className="spinner" aria-hidden /> {t("leftovers.searching")}
          </span>
        )}
        {list && list.length > 0 && (
          <>
            <button className="btn" disabled={chosen.length === 0} onClick={() => setConfirming(chosen)}>
              {t("trash.button")}
            </button>
            <span className="muted">
              {chosen.length} · <span className="num">{formatBytes(total(chosen), lang)}</span> / {formatBytes(total(list), lang)}
            </span>
          </>
        )}
      </div>
      {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      {list && list.length === 0 && <p className="muted">{t("leftovers.none")}</p>}
      {list && list.length > 0 && (
        <>
          <p className="muted small">{t("leftovers.warning")}</p>
          <ul className="items grid-leftovers">
            {list.map((l) => (
              <li key={l.path}>
                <label className="check path" title={l.trashable ? l.path : t("leftovers.adminOnly")}>
                  <input type="checkbox" checked={selected.has(l.path)} disabled={!l.trashable} onChange={() => toggle(l.path)} />
                  <span>{l.path}</span>
                </label>
                <span className="muted small">{t(`leftovers.place.${l.place}` as Key)}</span>
                <span className="muted small" title={t("leftovers.lastChange")}>
                  {formatDay(l.modified, lang)}
                </span>
                <span className="num end">{formatBytes(l.size, lang)}</span>
                <button className="btn btn-small" onClick={() => revealInExplorer(l.path)}>
                  {t("analyse.open")}
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
      <TrashConfirm
        items={confirming.map((l) => ({ path: l.path, size: l.size, modified: l.modified }))}
        onConfirm={trash}
        onCancel={() => setConfirming([])}
      />
    </Card>
  );
}
