import { useEffect, useState } from "react";
import { useT } from "../i18n/useT";
import { findGhosts, removeGhosts, type Ghost } from "../lib/api";
import { Card } from "./Card";
import { ConfirmDialog } from "./ConfirmDialog";
import { StatusPill } from "./StatusPill";

/** Entrées de la liste des programmes sans désinstalleur ni dossier : sauvegardées en .reg puis retirées. */
export function Ghosts({ onChange }: { onChange: () => void }) {
  const { t } = useT();
  const [list, setList] = useState<Ghost[] | null>(null);
  const [running, setRunning] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirm, setConfirm] = useState(false);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  const chosen = list?.filter((g) => selected.has(g.id)) ?? [];

  // Lancée dès l'ouverture de la page (lecture seule) : pas besoin de cliquer.
  useEffect(() => {
    search();
  }, []);

  async function search() {
    setRunning(true);
    setMessage(null);
    try {
      setList(await findGhosts());
      setSelected(new Set());
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    } finally {
      setRunning(false);
    }
  }

  function toggle(id: string) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelected(next);
  }

  async function remove() {
    setConfirm(false);
    const ids = chosen.map((g) => g.id);
    try {
      const n = await removeGhosts(ids);
      setList((prev) => prev?.filter((g) => !ids.includes(g.id)) ?? null);
      setSelected(new Set());
      setMessage({ status: "ok", text: `${n} ${t("ghosts.removed")}` });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    } finally {
      onChange();
    }
  }

  return (
    <Card title={t("ghosts.title")} fold={{ id: "ghosts", summary: running && !list ? t("fold.searching") : list ? t("fold.found").replace("{n}", String(list.length)) : null }}>
      <p>{t("ghosts.intro")}</p>
      <div className="row">
        <button className="btn btn-primary" disabled={running} onClick={search}>
          {list ? t("dups.again") : t("ghosts.search")}
        </button>
        {list && list.length > 0 && (
          <button className="btn" disabled={chosen.length === 0} onClick={() => setConfirm(true)}>
            {t("ghosts.remove")} ({chosen.length})
          </button>
        )}
      </div>
      {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      {list && list.length === 0 && <p className="muted">{t("ghosts.none")}</p>}
      {list && list.length > 0 && (
        <ul className="items grid-ghosts">
          {list.map((g) => (
            <li key={g.id}>
              <label className="check path" title={g.command}>
                <input type="checkbox" checked={selected.has(g.id)} onChange={() => toggle(g.id)} />
                <span>
                  {g.name}
                  <span className="block muted small">{g.command}</span>
                </span>
              </label>
              <span className="muted small">{g.publisher || "—"}</span>
              <span className="muted small">{g.version}</span>
            </li>
          ))}
        </ul>
      )}
      <ConfirmDialog
        open={confirm}
        title={t("ghosts.confirmTitle")}
        confirmLabel={t("ghosts.remove")}
        cancelLabel={t("common.cancel")}
        onConfirm={remove}
        onCancel={() => setConfirm(false)}
      >
        <ul className="paths">
          {chosen.map((g) => (
            <li key={g.id}>{g.name}</li>
          ))}
        </ul>
        <p className="ok">{t("ghosts.confirm")}</p>
      </ConfirmDialog>
    </Card>
  );
}
