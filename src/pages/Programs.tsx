import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { SortHeader } from "../components/SortHeader";
import { StatusPill } from "../components/StatusPill";
import { useT } from "../i18n/useT";
import { listPrograms, programSizes, uninstallProgram, type Program } from "../lib/api";
import { formatBytes } from "../lib/format";
import { sortBy, type SortState } from "../lib/sort";

function formatInstallDate(d: string, lang: string): string {
  if (!/^\d{8}$/.test(d)) return "";
  return new Date(+d.slice(0, 4), +d.slice(4, 6) - 1, +d.slice(6, 8)).toLocaleDateString(lang);
}

export function Programs() {
  const { t, lang } = useT();
  const [programs, setPrograms] = useState<Program[] | null>(null);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SortState<"measured" | "declared" | "name" | "date" | "publisher">>({
    key: "measured",
    dir: "desc",
  });
  const [showDeclared, setShowDeclared] = useState(true);
  const [showMeasured, setShowMeasured] = useState(true);
  const [target, setTarget] = useState<Program | null>(null);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  const [real, setReal] = useState<Map<string, number> | null>(null);

  function load() {
    setPrograms(null);
    setReal(null);
    listPrograms().then(setPrograms, (e) => setMessage({ status: "danger", text: String(e) }));
    // Taille réelle mesurée sur le disque, arrive après la liste.
    programSizes().then((s) => setReal(new Map(s.map((r) => [r.id, r.bytes]))), () => setReal(new Map()));
  }

  // Taille mesurée inconnue : -1 pour la ranger après les tailles connues.
  const measured = (p: Program) => real?.get(p.id) ?? -1;

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
  const shown =
    programs &&
    sortBy(
      programs.filter((p) => !q || `${p.name} ${p.publisher}`.toLowerCase().includes(q)),
      sort,
      { measured, declared: (p) => p.bytes, name: (p) => p.name, date: (p) => p.install_date, publisher: (p) => p.publisher },
    );

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
        <div className="row">
          <label className="check">
            <input
              type="checkbox"
              checked={showMeasured}
              disabled={showMeasured && !showDeclared}
              onChange={(e) => {
                setShowMeasured(e.target.checked);
                if (!e.target.checked && sort.key === "measured") setSort({ key: "declared", dir: "desc" });
              }}
            />
            {t("programs.showMeasured")}
          </label>
          <label className="check">
            <input
              type="checkbox"
              checked={showDeclared}
              disabled={showDeclared && !showMeasured}
              onChange={(e) => {
                setShowDeclared(e.target.checked);
                if (!e.target.checked && sort.key === "declared") setSort({ key: "measured", dir: "desc" });
              }}
            />
            {t("programs.showDeclared")}
          </label>
        </div>
        {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      </Card>

      <Card>
        {real === null && (
          <span className="loading" role="status">
            <span className="spinner" aria-hidden /> {t("programs.measuring")}
          </span>
        )}
        {!shown ? (
          <p className="muted">…</p>
        ) : (
          <ul className="items programs grid-programs">
            <SortHeader
              cells={[
                {
                  sorts: [
                    ["name", t("sort.name"), false],
                    ["publisher", t("sort.publisher"), false],
                    ["date", t("sort.date"), true],
                  ],
                },
                {
                  sorts: [
                    ...(showMeasured ? [["measured", t("programs.measured"), true] as ["measured", string, boolean]] : []),
                    ...(showDeclared ? [["declared", t("programs.declaredCol"), true] as ["declared", string, boolean]] : []),
                  ],
                  end: true,
                },
                null,
              ]}
              value={sort}
              onChange={setSort}
            />
            {shown.map((p) => (
              <li key={p.id}>
                <div className="program">
                  <strong>{p.name}</strong>
                  <span className="muted">
                    {[p.publisher, p.version, formatInstallDate(p.install_date, lang)].filter(Boolean).join(" · ")}
                  </span>
                  {p.location && (
                    <span className="muted path" title={p.location}>
                      {p.location}
                    </span>
                  )}
                </div>
                <span className="sizes">
                  {showMeasured && (
                    <span className="size-cell">
                      <span className="num">
                        {real === null ? "…" : real.has(p.id) ? formatBytes(real.get(p.id)!, lang) : "—"}
                      </span>
                      <span className="muted small">{t("programs.real")}</span>
                    </span>
                  )}
                  {showDeclared && (
                    <span className="size-cell">
                      <span className="num">{p.bytes > 0 ? formatBytes(p.bytes, lang) : "—"}</span>
                      <span className="muted small">{t("programs.declared")}</span>
                    </span>
                  )}
                </span>
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
