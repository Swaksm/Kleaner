import { useEffect, useState } from "react";
import { BugReportButton } from "../components/BugReportButton";
import { Card } from "../components/Card";
import { StatusPill } from "../components/StatusPill";
import { useT } from "../i18n/useT";
import { moveBack, readJournal, restoreGhost, restoreTrashed, revealInExplorer, type JournalEntry, type JournalView } from "../lib/api";
import { formatBytes, formatDate } from "../lib/format";

/** Actions qu'on peut annuler : Corbeille, entrée fantôme sauvegardée, dossier déplacé vers un autre disque. */
const RESTORABLE = ["trash", "trash-leftovers", "ghost-remove", "move"];

export function Journal() {
  const { t, lang } = useT();
  const [view, setView] = useState<JournalView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [restored, setRestored] = useState<Map<number, string>>(new Map());

  function load() {
    readJournal().then(setView, (e) => setError(String(e)));
  }

  useEffect(load, []);

  async function restore(e: JournalEntry, key: number) {
    try {
      if (e.action === "move") {
        setRestored((m) => new Map(m).set(key, t("move.moving")));
        await moveBack(e.paths[0], e.paths[1]);
        setRestored((m) => new Map(m).set(key, t("move.backDone")));
      } else if (e.action === "ghost-remove" && e.output) {
        await restoreGhost(e.output);
        setRestored((m) => new Map(m).set(key, `1 ${t("journal.restoredCount")}`));
      } else {
        const r = await restoreTrashed(e.paths);
        setRestored((m) => new Map(m).set(key, `${r.files} ${t("journal.restoredCount")}`));
      }
      load();
    } catch (err) {
      setRestored((m) => new Map(m).set(key, String(err)));
    }
  }

  // Dossier déjà remis à sa place : plus de bouton.
  const movedBack = (e: JournalEntry) =>
    e.action === "move" &&
    !!view?.entries.some((b) => b.action === "move-back" && !b.error && b.ts >= e.ts && b.paths[0] === e.paths[1] && b.paths[1] === e.paths[0]);

  return (
    <>
      <p className="muted">{t("journal.intro")}</p>
      <div className="row">
        {view && (
          <button className="btn" onClick={() => revealInExplorer(view.path)}>
            {t("journal.openFile")}
          </button>
        )}
        <BugReportButton />
      </div>
      {error && <StatusPill status="danger">{error}</StatusPill>}
      {view?.entries.length === 0 && (
        <Card>
          <p>{t("journal.empty")}</p>
        </Card>
      )}
      {view?.entries.map((e, i) => (
        <Card key={`${e.ts}-${i}`}>
          <div className="row">
            <strong>{e.action}</strong>
            <span className="muted">{formatDate(e.ts, lang)}</span>
            {e.error ? (
              <StatusPill status="danger">{t("journal.failed")}</StatusPill>
            ) : (
              <StatusPill status="ok">{t("journal.done")}</StatusPill>
            )}
            {RESTORABLE.includes(e.action) && !e.error && !movedBack(e) && (
              <button className="btn btn-small" disabled={restored.has(e.ts)} onClick={() => restore(e, e.ts)}>
                {t("journal.restore")}
              </button>
            )}
            {restored.has(e.ts) && <span className="muted small">{restored.get(e.ts)}</span>}
          </div>
          <div className="row muted">
            <span>
              <span className="num">{e.files}</span> {t("journal.files")}
            </span>
            <span className="num">{formatBytes(e.bytes, lang)}</span>
            {e.skipped > 0 && (
              <span>
                <span className="num">{e.skipped}</span> {t("journal.skipped")}
              </span>
            )}
          </div>
          {(e.action === "move" || e.action === "move-back") && e.paths.length === 2 && (
            <p className="muted small path">
              {e.paths[0]} → {e.paths[1]}
            </p>
          )}
          {e.error && <p className="danger">{e.error}</p>}
        </Card>
      ))}
    </>
  );
}
