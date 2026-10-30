import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { EmptyBin } from "../components/EmptyBin";
import { StatusPill } from "../components/StatusPill";
import { TrashConfirm } from "../components/TrashConfirm";
import { useT, type Key } from "../i18n/useT";
import { findAdvice, revealInExplorer, trashAdvice, uninstallSteamGame, type Advice as Tip } from "../lib/api";
import { formatBytes, formatDay } from "../lib/format";

const KINDS: Tip["kind"][] = ["installer", "archive", "game"];

/** Conseils sur des règles simples : installeurs anciens, archives déjà extraites, jeux Steam délaissés. */
export function Advice() {
  const { t, lang } = useT();
  const [list, setList] = useState<Tip[] | null>(null);
  const [running, setRunning] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState<Tip[]>([]);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string; bin?: boolean } | null>(null);

  const chosen = list?.filter((a) => a.kind !== "game" && selected.has(a.path)) ?? [];

  // Lancée dès l'ouverture de la page (lecture seule) : pas besoin de cliquer.
  useEffect(() => {
    search();
  }, []);

  async function search() {
    setRunning(true);
    setMessage(null);
    try {
      setList(await findAdvice());
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
      const r = await trashAdvice(items.map((a) => a.path));
      const gone = new Set(items.map((a) => a.path));
      setList((prev) => prev?.filter((a) => !gone.has(a.path)) ?? null);
      setSelected(new Set());
      setMessage({ status: "ok", bin: true, text: `${r.files} — ${t("trash.done")}` });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  return (
    <>
      <Card>
        <p>{t("advice.intro")}</p>
        <div className="row">
          <button className="btn btn-primary" disabled={running} onClick={search}>
            {list ? t("dups.again") : t("advice.search")}
          </button>
          {running && (
            <span className="loading" role="status">
              <span className="spinner" aria-hidden /> {t("advice.searching")}
            </span>
          )}
          {list && list.some((a) => a.kind !== "game") && (
            <>
              <button className="btn" disabled={chosen.length === 0} onClick={() => setConfirming(chosen)}>
                {t("trash.button")}
              </button>
              <span className="muted">{chosen.length}</span>
            </>
          )}
        </div>
        {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
        {message?.bin && <EmptyBin />}
        {list && list.length === 0 && <p className="muted">{t("advice.none")}</p>}
      </Card>

      {KINDS.map((kind) => {
        const items = list?.filter((a) => a.kind === kind) ?? [];
        if (items.length === 0) return null;
        return (
          <Card key={kind} title={t(`advice.${kind}` as Key)}>
            <p className="muted">{t(`advice.${kind}Hint` as Key)}</p>
            <ul className="items grid-shortcuts">
              {items.map((a) => (
                <li key={a.path}>
                  {a.app_id ? (
                    <span className="path" title={a.path}>
                      {a.name}
                      <span className="block muted small">{a.path}</span>
                    </span>
                  ) : (
                    <label className="check path" title={a.path}>
                      <input type="checkbox" checked={selected.has(a.path)} onChange={() => toggle(a.path)} />
                      <span>{a.path}</span>
                    </label>
                  )}
                  <span className="num">{formatBytes(a.size, lang)}</span>
                  <span className="muted small">
                    {a.app_id ? `${t("advice.lastPlayed")} ` : ""}
                    {formatDay(a.modified, lang)}
                  </span>
                  {a.app_id ? (
                    <button
                      className="btn btn-small"
                      onClick={() => uninstallSteamGame(a.app_id!).catch((e) => setMessage({ status: "danger", text: String(e) }))}
                    >
                      {t("advice.uninstall")}
                    </button>
                  ) : (
                    <button className="btn btn-small" onClick={() => revealInExplorer(a.path)}>
                      {t("analyse.open")}
                    </button>
                  )}
                </li>
              ))}
            </ul>
          </Card>
        );
      })}

      <TrashConfirm
        items={confirming.map((a) => ({ path: a.path, size: a.size, modified: a.modified }))}
        onConfirm={trash}
        onCancel={() => setConfirming([])}
      />
    </>
  );
}
