import { useState } from "react";
import { useT, type Key } from "../i18n/useT";
import { findPreinstalled, openStorePage, removePreinstalled, type PreinstalledApp } from "../lib/api";
import { formatBytes } from "../lib/format";
import { Card } from "./Card";
import { ConfirmDialog } from "./ConfirmDialog";
import { StatusPill } from "./StatusPill";

/** Applis préinstallées d'une liste fermée, retirées pour ce compte seulement. Rien n'est coché d'office. */
export function Preinstalled() {
  const { t, lang } = useT();
  const [list, setList] = useState<PreinstalledApp[] | null>(null);
  const [running, setRunning] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirm, setConfirm] = useState(false);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);
  // Retirées pendant cette visite : bouton pour les réinstaller depuis le Store.
  const [removed, setRemoved] = useState<PreinstalledApp[]>([]);

  const chosen = list?.filter((a) => selected.has(a.id)) ?? [];

  async function search() {
    setRunning(true);
    setMessage(null);
    try {
      setList(await findPreinstalled());
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
    setRunning(true);
    const apps = chosen;
    try {
      const report = await removePreinstalled(apps.map((a) => a.id));
      const done = apps.filter((a) => report.some((r) => r.id === a.id && !r.error));
      const failed = report.filter((r) => r.error);
      setRemoved((prev) => [...prev, ...done]);
      setList((prev) => prev?.filter((a) => !done.includes(a)) ?? null);
      setSelected(new Set());
      setMessage(
        failed.length === 0
          ? { status: "ok", text: `${done.length} ${t("debloat.removed")}` }
          : { status: "danger", text: `${failed.length} ${t("debloat.failed")} ${failed.map((f) => f.error).join(" · ")}` },
      );
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    } finally {
      setRunning(false);
    }
  }

  return (
    <Card title={t("debloat.title")}>
      <p>{t("debloat.intro")}</p>
      <div className="row">
        <button className="btn btn-primary" disabled={running} onClick={search}>
          {list ? t("dups.again") : t("debloat.search")}
        </button>
        {list && list.length > 0 && (
          <button className="btn" disabled={chosen.length === 0 || running} onClick={() => setConfirm(true)}>
            {t("debloat.remove")} ({chosen.length})
          </button>
        )}
        {running && <span className="spinner" role="status" aria-label={t("debloat.search")} />}
      </div>
      {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      {list && list.length === 0 && <p className="muted">{t("debloat.none")}</p>}
      {list && list.length > 0 && (
        <ul className="items grid-preinstalled">
          {list.map((a) => (
            <li key={a.id}>
              <label className="check path" title={a.id}>
                <input type="checkbox" checked={selected.has(a.id)} onChange={() => toggle(a.id)} />
                <span>
                  {a.name}
                  <span className="block muted small">{a.package}</span>
                  {a.warn && <span className="block warn small">{t(`debloat.warn.${a.warn}` as Key)}</span>}
                </span>
              </label>
              <span className="num">{a.size > 0 ? formatBytes(a.size, lang) : "—"}</span>
            </li>
          ))}
        </ul>
      )}
      {removed.length > 0 && (
        <>
          <p className="muted small">{t("debloat.reinstallHint")}</p>
          <ul className="items grid-preinstalled">
            {removed.map((a) => (
              <li key={a.id}>
                <span>{a.name}</span>
                <button className="btn" onClick={() => openStorePage(a.family).catch((e) => setMessage({ status: "danger", text: String(e) }))}>
                  {t("debloat.reinstall")}
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
      <ConfirmDialog
        open={confirm}
        title={t("debloat.confirmTitle")}
        confirmLabel={t("debloat.remove")}
        cancelLabel={t("common.cancel")}
        danger
        onConfirm={remove}
        onCancel={() => setConfirm(false)}
      >
        <ul className="paths">
          {chosen.map((a) => (
            <li key={a.id}>
              {a.name}
              {a.warn && <span className="block warn small">{t(`debloat.warn.${a.warn}` as Key)}</span>}
            </li>
          ))}
        </ul>
        <p>{t("debloat.confirm")}</p>
      </ConfirmDialog>
    </Card>
  );
}
