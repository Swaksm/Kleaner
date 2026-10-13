import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { StatusPill } from "../components/StatusPill";
import { useT } from "../i18n/useT";
import { readJournal, revealInExplorer, type JournalView } from "../lib/api";
import { formatBytes, formatDate } from "../lib/format";

export function Journal() {
  const { t, lang } = useT();
  const [view, setView] = useState<JournalView | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    readJournal().then(setView, (e) => setError(String(e)));
  }, []);

  return (
    <>
      <p className="muted">{t("journal.intro")}</p>
      {view && (
        <button className="btn" onClick={() => revealInExplorer(view.path)}>
          {t("journal.openFile")}
        </button>
      )}
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
