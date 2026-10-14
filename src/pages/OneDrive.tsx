import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { SortHeader } from "../components/SortHeader";
import { StatusPill } from "../components/StatusPill";
import { useT } from "../i18n/useT";
import { onedriveFiles, onedriveFree, revealInExplorer, type CloudFile } from "../lib/api";
import { formatBytes, formatDay } from "../lib/format";
import { baseName, sortBy, type SortState } from "../lib/sort";

type Key = "size" | "name" | "date";

/** Gros fichiers OneDrive gardés sur le PC : « Libérer de l'espace » les laisse en ligne uniquement. */
export function OneDrive() {
  const { t, lang } = useT();
  const [files, setFiles] = useState<CloudFile[] | null>(null);
  const [sort, setSort] = useState<SortState<Key>>({ key: "size", dir: "desc" });
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirm, setConfirm] = useState(false);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  function load() {
    setFiles(null);
    onedriveFiles().then(setFiles, (e) => setMessage({ status: "danger", text: String(e) }));
  }

  useEffect(load, []);

  const shown = files ? sortBy(files, sort, { size: (f) => f.size, name: (f) => baseName(f.path), date: (f) => f.modified }) : [];
  const chosen = files?.filter((f) => selected.has(f.path)) ?? [];
  const sum = (l: CloudFile[]) => l.reduce((s, f) => s + f.size, 0);

  function toggle(path: string) {
    const next = new Set(selected);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setSelected(next);
  }

  async function free() {
    setConfirm(false);
    try {
      const r = await onedriveFree(chosen.map((f) => f.path));
      const gone = new Set(chosen.map((f) => f.path));
      setFiles((prev) => prev?.filter((f) => !gone.has(f.path)) ?? null);
      setSelected(new Set());
      setMessage({ status: "ok", text: `${r.files} · ${formatBytes(r.bytes, lang)} — ${t("onedrive.freed")}` });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  return (
    <>
      <Card>
        <p>{t("onedrive.intro")}</p>
        <div className="row">
          <button className="btn" onClick={() => setSelected(new Set(shown.filter((f) => !f.pinned).map((f) => f.path)))}>
            {t("media.selectVisible")}
          </button>
          <button className="btn" disabled={selected.size === 0} onClick={() => setSelected(new Set())}>
            {t("cleanup.selectNone")}
          </button>
          <button className="btn btn-primary" disabled={chosen.length === 0} onClick={() => setConfirm(true)}>
            {t("onedrive.free")}
          </button>
          <span className="muted">
            {chosen.length} · <span className="num">{formatBytes(sum(chosen), lang)}</span>
            {files && ` / ${formatBytes(sum(files), lang)}`}
          </span>
          <button className="btn" onClick={load}>
            {t("cleanup.refresh")}
          </button>
        </div>
        {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      </Card>

      <Card>
        {!files ? (
          <span className="loading" role="status">
            <span className="spinner" aria-hidden /> {t("cleanup.estimating")}
          </span>
        ) : files.length === 0 ? (
          <p className="muted">{t("onedrive.none")}</p>
        ) : (
          <ul className="items grid-4">
            <SortHeader
              cells={[
                { sorts: [["name", t("sort.name"), false]] },
                { sorts: [["date", t("sort.date"), true]] },
                { sorts: [["size", t("sort.size"), true]], end: true },
                null,
              ]}
              value={sort}
              onChange={setSort}
            />
            {shown.map((f) => (
              <li key={f.path}>
                <label className="check path" title={f.pinned ? t("onedrive.pinned") : f.path}>
                  <input type="checkbox" checked={selected.has(f.path)} onChange={() => toggle(f.path)} />
                  <span>
                    {f.path}
                    {f.provider !== "OneDrive" && <span className="muted small"> · {f.provider}</span>}
                    {f.pinned && <span className="muted small"> · {t("onedrive.pinnedShort")}</span>}
                  </span>
                </label>
                <span className="muted small">{formatDay(f.modified, lang)}</span>
                <span className="num end">{formatBytes(f.size, lang)}</span>
                <button className="btn btn-small" onClick={() => revealInExplorer(f.path)}>
                  {t("analyse.open")}
                </button>
              </li>
            ))}
          </ul>
        )}
      </Card>

      <ConfirmDialog
        open={confirm}
        title={t("onedrive.confirmTitle")}
        confirmLabel={t("onedrive.free")}
        cancelLabel={t("common.cancel")}
        onConfirm={free}
        onCancel={() => setConfirm(false)}
      >
        <p>
          {chosen.length} · <span className="num">{formatBytes(sum(chosen), lang)}</span>
        </p>
        <p className="ok">{t("onedrive.safe")}</p>
      </ConfirmDialog>
    </>
  );
}
