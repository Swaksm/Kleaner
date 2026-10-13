import { useState } from "react";
import { Card } from "../components/Card";
import { SortBar } from "../components/SortBar";
import { StatusPill } from "../components/StatusPill";
import { TrashConfirm } from "../components/TrashConfirm";
import { useT } from "../i18n/useT";
import { moveToTrash, revealInExplorer, type Item } from "../lib/api";
import { formatBytes, formatDay } from "../lib/format";
import { forgetPaths, useScanResult } from "../lib/scanStore";
import { baseName, sortBy, type SortState } from "../lib/sort";

type Kind = "videos" | "images";
type Key = "size" | "name" | "date" | "folder";
const PAGE = 300;

const getters: Record<Key, (i: Item) => string | number> = {
  size: (i) => i.size,
  name: (i) => baseName(i.path),
  date: (i) => i.modified,
  folder: (i) => i.path.slice(0, i.path.lastIndexOf("\\")),
};

export function Media() {
  const { t, lang } = useT();
  const scan = useScanResult();
  const [kind, setKind] = useState<Kind>("videos");
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SortState<Key>>({ key: "size", dir: "desc" });
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [limit, setLimit] = useState(PAGE);
  const [confirming, setConfirming] = useState<Item[]>([]);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  if (!scan) {
    return (
      <Card>
        <p>{t("media.needScan")}</p>
      </Card>
    );
  }

  const all = scan[kind];
  const q = query.trim().toLowerCase();
  const filtered = sortBy(
    all.filter((i) => !q || i.path.toLowerCase().includes(q)),
    sort,
    getters,
  );
  const visible = filtered.slice(0, limit);
  const chosen = all.filter((i) => selected.has(i.path));
  const total = (l: Item[]) => l.reduce((s, i) => s + i.size, 0);

  function switchKind(k: Kind) {
    setKind(k);
    setSelected(new Set());
    setLimit(PAGE);
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
      setMessage({ status: "ok", text: `${r.files} ${t("trash.files")} · ${formatBytes(r.bytes, lang)} — ${t("trash.done")}` });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  return (
    <>
      <Card>
        <p>{t("media.intro")}</p>
        <div className="row">
          {(["videos", "images"] as Kind[]).map((k) => (
            <button
              key={k}
              className={kind === k ? "btn btn-active" : "btn"}
              aria-pressed={kind === k}
              onClick={() => switchKind(k)}
            >
              {t(`media.${k}` as "media.videos")} · <span className="num">{scan[k].length.toLocaleString(lang)}</span> ·{" "}
              <span className="num">{formatBytes(total(scan[k]), lang)}</span>
            </button>
          ))}
        </div>
        <div className="row">
          <input
            className="search"
            type="search"
            placeholder={t("media.search")}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setLimit(PAGE);
            }}
          />
          <SortBar
            options={{
              size: [t("sort.size"), true],
              name: [t("sort.name"), false],
              date: [t("sort.date"), true],
              folder: [t("sort.folder"), false],
            }}
            value={sort}
            onChange={setSort}
          />
        </div>
        {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      </Card>

      <Card>
        <div className="row">
          <button className="btn btn-primary" disabled={chosen.length === 0} onClick={() => setConfirming(chosen)}>
            {t("trash.button")}
          </button>
          <span className="muted">
            {chosen.length} · <span className="num">{formatBytes(total(chosen), lang)}</span>
          </span>
          <button className="btn" onClick={() => setSelected(new Set([...selected, ...visible.map((i) => i.path)]))}>
            {t("media.selectVisible")}
          </button>
          <button className="btn" disabled={selected.size === 0} onClick={() => setSelected(new Set())}>
            {t("cleanup.selectNone")}
          </button>
        </div>
        <p className="muted">
          {filtered.length.toLocaleString(lang)} {t("media.results")} · {formatBytes(total(filtered), lang)}
        </p>
        <ul className="items media-list">
          {visible.map((i) => (
            <li key={i.path}>
              <label className="check media-name" title={i.path}>
                <input type="checkbox" checked={selected.has(i.path)} onChange={() => toggle(i.path)} />
                <span className="media-text">
                  <strong>{baseName(i.path)}</strong>
                  <span className="muted path">{i.path.slice(0, i.path.lastIndexOf("\\"))}</span>
                </span>
              </label>
              <span className="muted small">{formatDay(i.modified, lang)}</span>
              <span className="num">{formatBytes(i.size, lang)}</span>
              <button className="btn btn-small" onClick={() => revealInExplorer(i.path)}>
                {t("analyse.open")}
              </button>
            </li>
          ))}
        </ul>
        {filtered.length > limit && (
          <button className="btn" onClick={() => setLimit(limit + PAGE)}>
            {t("media.more")} ({(filtered.length - limit).toLocaleString(lang)})
          </button>
        )}
      </Card>

      <TrashConfirm items={confirming} onConfirm={trash} onCancel={() => setConfirming([])} />
    </>
  );
}
