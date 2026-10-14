import { useEffect, useState } from "react";
import { BugReportButton } from "../components/BugReportButton";
import { Card } from "../components/Card";
import { StatusPill } from "../components/StatusPill";
import { useT } from "../i18n/useT";
import { readJournal, restoreTrashed, revealInExplorer, type JournalEntry, type JournalView } from "../lib/api";
import { formatBytes, formatDate } from "../lib/format";

/** Actions qui passent par la Corbeille : on peut les annuler. */
const RESTORABLE = ["trash", "trash-leftovers"];

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
      const r = await restoreTrashed(e.paths);
      setRestored((m) => new Map(m).set(key, `${r.files} ${t("journal.restoredCount")}`));
      load();
    } catch (err) {
      setRestored((m) => new Map(m).set(key, String(err)));
    }
  }

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
            {RESTORABLE.includes(e.action) && !e.error && (
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
          {e.error && <p className="danger">{e.error}</p>}
        </Card>
      ))}
    </>
  );
}
