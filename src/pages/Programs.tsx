import { useEffect, useState } from "react";
import { Skeleton, SkeletonInline } from "../components/Skeleton";
import { Card } from "../components/Card";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { Ghosts } from "../components/Ghosts";
import { Leftovers } from "../components/Leftovers";
import { SortHeader, type HeaderCell } from "../components/SortHeader";
import { StatusPill } from "../components/StatusPill";
import { useT } from "../i18n/useT";
import { isElevated, listPrograms, programSizes, uninstallProgram, type Program } from "../lib/api";
import { formatBytes, formatDay } from "../lib/format";
import { sortBy, type SortState } from "../lib/sort";

const POLL_MS = 3000;
const WATCH_MS = 10 * 60 * 1000;

type SortKey = "measured" | "declared" | "name" | "date" | "publisher" | "used";

function formatInstallDate(d: string, lang: string): string {
  if (!/^\d{8}$/.test(d)) return "";
  return new Date(+d.slice(0, 4), +d.slice(4, 6) - 1, +d.slice(6, 8)).toLocaleDateString(lang);
}

export function Programs() {
  const { t, lang } = useT();
  const [programs, setPrograms] = useState<Program[] | null>(null);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SortState<SortKey>>({
    key: "measured",
    dir: "desc",
  });
  const [showDeclared, setShowDeclared] = useState(true);
  const [showMeasured, setShowMeasured] = useState(true);
  const [elevated, setElevated] = useState(false);
  const [target, setTarget] = useState<Program | null>(null);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  const [real, setReal] = useState<Map<string, number> | null>(null);
  const [lastUsed, setLastUsed] = useState<Map<string, number>>(new Map());
  // Programmes dont le désinstalleur a été lancé : id → heure de lancement.
  const [watching, setWatching] = useState<Map<string, number>>(new Map());

  // Le désinstalleur tourne hors de Kleaner : on relit le registre jusqu'à ce que le programme disparaisse.
  useEffect(() => {
    if (watching.size === 0) return;
    const timer = setInterval(async () => {
      let list: Program[];
      try {
        list = await listPrograms();
      } catch {
        return;
      }
      const present = new Set(list.map((p) => p.id));
      const now = Date.now();
      const gone = [...watching.keys()].filter((id) => !present.has(id));
      const expired = [...watching].filter(([id, start]) => present.has(id) && now - start > WATCH_MS).map(([id]) => id);
      if (gone.length === 0 && expired.length === 0) return;
      setPrograms((prev) => {
        const names = prev?.filter((p) => gone.includes(p.id)).map((p) => p.name) ?? [];
        if (names.length > 0) setMessage({ status: "ok", text: `${names.join(", ")} : ${t("programs.removed")}` });
        return prev?.filter((p) => !gone.includes(p.id)) ?? prev;
      });
      setWatching((prev) => {
        const next = new Map(prev);
        [...gone, ...expired].forEach((id) => next.delete(id));
        return next;
      });
    }, POLL_MS);
    return () => clearInterval(timer);
  }, [watching, t]);

  function load() {
    setPrograms(null);
    setReal(null);
    listPrograms().then(setPrograms, (e) => setMessage({ status: "danger", text: String(e) }));
    // Taille réelle mesurée sur le disque, arrive après la liste.
    programSizes().then(
      (s) => {
        setReal(new Map(s.map((r) => [r.id, r.bytes])));
        setLastUsed(new Map(s.filter((r) => r.last_used > 0).map((r) => [r.id, r.last_used])));
      },
      () => setReal(new Map()),
    );
  }

  // Taille mesurée inconnue : -1 pour la ranger après les tailles connues.
  const measured = (p: Program) => real?.get(p.id) ?? -1;

  useEffect(load, []);
  useEffect(() => {
    isElevated().then(setElevated, () => {});
  }, []);

  async function uninstall() {
    const p = target;
    setTarget(null);
    if (!p) return;
    try {
      await uninstallProgram(p.id);
      setWatching((prev) => new Map(prev).set(p.id, Date.now()));
      setMessage({ status: "ok", text: `${p.name} : ${t("programs.launched")}` });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  // Colonnes : nom, éditeur, installé, dernier lancement, [mesurée], [déclarée], action.
  const cols = ["minmax(140px, 1fr)", "120px", "80px", "84px", ...(showMeasured ? ["78px"] : []), ...(showDeclared ? ["78px"] : []), "100px"].join(" ");
  const q = query.trim().toLowerCase();
  const shown =
    programs &&
    sortBy(
      programs.filter((p) => !q || `${p.name} ${p.publisher}`.toLowerCase().includes(q)),
      sort,
      {
        measured,
        declared: (p) => p.bytes,
        name: (p) => p.name,
        date: (p) => p.install_date,
        publisher: (p) => p.publisher,
        used: (p) => lastUsed.get(p.id) ?? 0,
      },
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

      {/* Avant la longue liste : sinon on ne la voit pas. */}
      <Leftovers />
      <Ghosts onChange={load} />

      <Card>
        {real === null && (
          <span className="loading" role="status">
            <span className="spinner" aria-hidden /> {t("programs.measuring")}
          </span>
        )}
        {!shown ? (
          <Skeleton lines={8} />
        ) : (
          <ul className="items programs" style={{ ["--cols" as string]: cols }}>
            <SortHeader
              cells={[
                { sorts: [["name", t("sort.name"), false]] },
                { sorts: [["publisher", t("sort.publisher"), false]] },
                { sorts: [["date", t("programs.installed"), true]] },
                { sorts: [["used", t("programs.lastUsed"), true]] },
                ...(showMeasured ? [{ sorts: [["measured", t("programs.measured"), true]], end: true } as HeaderCell<SortKey>] : []),
                ...(showDeclared ? [{ sorts: [["declared", t("programs.declaredCol"), true]], end: true } as HeaderCell<SortKey>] : []),
                null,
              ]}
              value={sort}
              onChange={setSort}
            />
            {shown.map((p) => (
              <li key={p.id}>
                <div className="program">
                  <strong>{p.name}</strong>
                  {p.location && (
                    <span className="muted path small" title={p.location}>
                      {p.location}
                    </span>
                  )}
                </div>
                <span className="muted small cell-clip" title={p.publisher}>
                  {p.publisher || "—"}
                  {p.version && <span className="block">{p.version}</span>}
                </span>
                <span className="muted small">{formatInstallDate(p.install_date, lang) || "—"}</span>
                <span className="muted small" title={elevated ? "" : t("programs.lastUsedAdmin")}>
                  {lastUsed.has(p.id) ? formatDay(lastUsed.get(p.id)!, lang) : "—"}
                </span>
                {showMeasured && (
                  <span className="num end">{real === null ? <SkeletonInline /> : real.has(p.id) ? formatBytes(real.get(p.id)!, lang) : "—"}</span>
                )}
                {showDeclared && <span className="num end">{p.bytes > 0 ? formatBytes(p.bytes, lang) : "—"}</span>}
                {watching.has(p.id) ? (
                  <span className="loading small" role="status">
                    <span className="spinner" aria-hidden /> {t("programs.uninstalling")}
                  </span>
                ) : (
                  <button className="btn btn-small" onClick={() => setTarget(p)}>
                    {t("programs.uninstall")}
                  </button>
                )}
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
