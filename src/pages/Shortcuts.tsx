import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { EmptyBin } from "../components/EmptyBin";
import { StatusPill } from "../components/StatusPill";
import { TrashConfirm } from "../components/TrashConfirm";
import { useT, type Key } from "../i18n/useT";
import { findShortcuts, isElevated, revealInExplorer, trashShortcuts, type BrokenShortcut } from "../lib/api";
import { formatDay } from "../lib/format";

/** Raccourcis du Bureau, du menu Démarrer et de la barre des tâches dont la cible n'existe plus. */
export function Shortcuts() {
  const { t, lang } = useT();
  const [list, setList] = useState<BrokenShortcut[] | null>(null);
  const [running, setRunning] = useState(false);
  const [elevated, setElevated] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState<BrokenShortcut[]>([]);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string; bin?: boolean } | null>(null);

  useEffect(() => {
    isElevated().then(setElevated, () => {});
  }, []);

  const usable = (s: BrokenShortcut) => !s.machine || elevated;
  const chosen = list?.filter((s) => selected.has(s.path) && usable(s)) ?? [];

  async function search() {
    setRunning(true);
    setMessage(null);
    try {
      const found = await findShortcuts();
      setList(found);
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
      const r = await trashShortcuts(items.map((s) => s.path));
      const gone = new Set(items.map((s) => s.path));
      setList((prev) => prev?.filter((s) => !gone.has(s.path)) ?? null);
      setSelected(new Set());
      setMessage({ status: "ok", bin: true, text: `${r.files} — ${t("trash.done")}` });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  return (
    <>
      <Card>
        <p>{t("shortcuts.intro")}</p>
        <div className="row">
          <button className="btn btn-primary" disabled={running} onClick={search}>
            {list ? t("dups.again") : t("shortcuts.search")}
          </button>
          {running && (
            <span className="loading" role="status">
              <span className="spinner" aria-hidden /> {t("shortcuts.searching")}
            </span>
          )}
          {list && list.length > 0 && (
            <>
              <button className="btn" onClick={() => setSelected(new Set(list.filter(usable).map((s) => s.path)))}>
                {t("media.selectVisible")}
              </button>
              <button className="btn" disabled={chosen.length === 0} onClick={() => setConfirming(chosen)}>
                {t("trash.button")}
              </button>
              <span className="muted">{chosen.length}</span>
            </>
          )}
        </div>
        {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
        {message?.bin && <EmptyBin />}
        {list && list.length === 0 && <p className="muted">{t("shortcuts.none")}</p>}
      </Card>

      {list && list.length > 0 && (
        <Card>
          <ul className="items grid-shortcuts">
            {list.map((s) => (
              <li key={s.path}>
                <label className="check path" title={usable(s) ? s.path : t("shortcuts.adminOnly")}>
                  <input type="checkbox" checked={selected.has(s.path)} disabled={!usable(s)} onChange={() => toggle(s.path)} />
                  <span>
                    {s.path}
                    <span className="block muted small">
                      {t("shortcuts.target")} {s.target}
                    </span>
                  </span>
                </label>
                <span className="muted small">{t(`shortcuts.place.${s.place}` as Key)}</span>
                <span className="muted small">{formatDay(s.modified, lang)}</span>
                <button className="btn btn-small" onClick={() => revealInExplorer(s.path)}>
                  {t("analyse.open")}
                </button>
              </li>
            ))}
          </ul>
        </Card>
      )}

      <TrashConfirm
        items={confirming.map((s) => ({ path: s.path, size: 0, modified: s.modified }))}
        onConfirm={trash}
        onCancel={() => setConfirming([])}
      />
    </>
  );
}
