import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { StatusPill } from "../components/StatusPill";
import { useT } from "../i18n/useT";
import { listPrograms, uninstallProgram, type Program } from "../lib/api";
import { formatBytes } from "../lib/format";

function formatInstallDate(d: string, lang: string): string {
  if (!/^\d{8}$/.test(d)) return "";
  return new Date(+d.slice(0, 4), +d.slice(4, 6) - 1, +d.slice(6, 8)).toLocaleDateString(lang);
}

export function Programs() {
  const { t, lang } = useT();
  const [programs, setPrograms] = useState<Program[] | null>(null);
  const [query, setQuery] = useState("");
  const [target, setTarget] = useState<Program | null>(null);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  function load() {
    setPrograms(null);
    listPrograms().then(setPrograms, (e) => setMessage({ status: "danger", text: String(e) }));
  }

  useEffect(load, []);

  async function uninstall() {
    const p = target;
    setTarget(null);
    if (!p) return;
    try {
      await uninstallProgram(p.id);
      setMessage({ status: "ok", text: `${p.name} : ${t("programs.launched")}` });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  const q = query.trim().toLowerCase();
  const shown = programs?.filter((p) => !q || `${p.name} ${p.publisher}`.toLowerCase().includes(q));

  return (
    <>
      <Card>
        <p>{t("programs.intro")}</p>
        <div className="row">
          <input
            className="search"
            type="search"
            placeholder={t("programs.search")}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button className="btn" onClick={load}>
            {t("cleanup.refresh")}
          </button>
        </div>
        {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      </Card>

      <Card>
        {!shown ? (
          <p className="muted">…</p>
        ) : (
          <ul className="items programs">
            {shown.map((p) => (
              <li key={p.id}>
                <div className="program">
                  <strong>{p.name}</strong>
                  <span className="muted">
                    {[p.publisher, p.version, formatInstallDate(p.install_date, lang)].filter(Boolean).join(" · ")}
                  </span>
                </div>
                <span className="num">{p.bytes > 0 ? formatBytes(p.bytes, lang) : "—"}</span>
                <button className="btn btn-small" onClick={() => setTarget(p)}>
                  {t("programs.uninstall")}
                </button>
              </li>
            ))}
          </ul>
        )}
      </Card>

      <ConfirmDialog
        open={target !== null}
        title={`${t("programs.uninstall")} ${target?.name ?? ""} ?`}
        confirmLabel={t("programs.uninstall")}
        cancelLabel={t("common.cancel")}
        onConfirm={uninstall}
        onCancel={() => setTarget(null)}
      >
        <p>{t("programs.confirm")}</p>
      </ConfirmDialog>
    </>
  );
}
