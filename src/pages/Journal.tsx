import { useEffect, useState } from "react";
import { BugReportButton } from "../components/BugReportButton";
import { Card } from "../components/Card";
import { StatusPill } from "../components/StatusPill";
import { useT, type Key } from "../i18n/useT";
import {
  listCleaners,
  moveBack,
  readJournal,
  restoreGhost,
  restoreTrashed,
  revealInExplorer,
  type Text,
  type JournalEntry,
  type JournalView,
} from "../lib/api";
import { formatBytes } from "../lib/format";

/** Actions qu'on peut annuler : Corbeille, entrée fantôme sauvegardée, dossier déplacé vers un autre disque. */
const RESTORABLE = ["trash", "trash-leftovers", "ghost-remove", "move"];

/** Actions qui ont un nom et une description dans les textes (`journal.action.*`, `journal.desc.*`). */
const KNOWN = [
  "trash", "trash-leftovers", "restore", "report", "autostart-on", "autostart-off", "autostart-limited", "clean",
  "remove-app", "move", "move-back", "move-start", "move-back-start", "cloud-free-up", "ghost-remove", "ghost-restore",
  "uninstall", "restore-point", "startup-enable", "startup-disable", "app-upgrade", "update",
];

/** « clean:browser-chrome-cache » → « clean » ; une action inconnue (journal d'une autre version) reste telle quelle. */
const kindOf = (action: string) => (action.startsWith("clean:") ? "clean" : action);

export function Journal() {
  const { t, lang, tx } = useT();
  const [view, setView] = useState<JournalView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [restored, setRestored] = useState<Map<string, string>>(new Map());
  const [names, setNames] = useState<Map<string, Text>>(new Map());

  function load() {
    readJournal().then(setView, (e) => setError(String(e)));
  }

  useEffect(load, []);
  // Noms des nettoyages pour les lignes « clean:… » ; sans eux, l'identifiant reste affiché.
  useEffect(() => {
    listCleaners().then((v) => setNames(new Map(v.cleaners.map((c) => [c.id, c.label]))), () => {});
  }, []);

  async function restore(e: JournalEntry, key: string) {
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

  function title(e: JournalEntry) {
    const kind = kindOf(e.action);
    if (kind === "clean") {
      const id = e.action.slice("clean:".length);
      return `${t("journal.action.clean")} — ${names.has(id) ? tx(names.get(id)!) : id}`;
    }
    return KNOWN.includes(kind) ? t(`journal.action.${kind}` as Key) : e.action;
  }

  // Une carte par jour, la plus récente en premier (le journal arrive déjà dans cet ordre).
  const days: { day: string; entries: JournalEntry[] }[] = [];
  for (const e of view?.entries ?? []) {
    const day = new Date(e.ts * 1000).toLocaleDateString(lang, { weekday: "long", day: "numeric", month: "long", year: "numeric" });
    if (days.length && days[days.length - 1].day === day) days[days.length - 1].entries.push(e);
    else days.push({ day, entries: [e] });
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
      {days.map(({ day, entries }, d) => (
        // Index dans la clé : un jour peut revenir si l'horloge a reculé.
        <Card key={`${day}-${d}`} title={day}>
          <ul className="journal">
            {entries.map((e, i) => {
              // Clé propre à la ligne : deux actions de la même seconde ne partagent pas leur bouton.
              const key = `${e.ts}|${e.action}|${e.paths.join("|")}`;
              const kind = kindOf(e.action);
              const canRestore = RESTORABLE.includes(e.action) && !e.error && !movedBack(e);
              const time = new Date(e.ts * 1000).toLocaleTimeString(lang, { hour: "2-digit", minute: "2-digit" });
              return (
                <li key={`${key}-${i}`}>
                  <details>
                    <summary>
                      <span className="num muted">{time}</span>
                      <strong className="journal-title">{title(e)}</strong>
                      <span className="muted small">
                        {e.files > 0 && (
                          <>
                            <span className="num">{e.files}</span> {t("journal.files")}
                          </>
                        )}
                        {e.bytes > 0 && <> · <span className="num">{formatBytes(e.bytes, lang)}</span></>}
                        {e.skipped > 0 && (
                          <>
                            {" "}
                            · <span className="num">{e.skipped}</span> {t("journal.skipped")}
                          </>
                        )}
                      </span>
                      {e.error ? (
                        <StatusPill status="danger">{t("journal.failed")}</StatusPill>
                      ) : (
                        <StatusPill status="ok">{t("journal.done")}</StatusPill>
                      )}
                      <span className="journal-restore">
                        {canRestore && (
                          <button
                            className="btn btn-small"
                            disabled={restored.has(key)}
                            onClick={(ev) => {
                              ev.preventDefault(); // le clic ne doit pas déplier la ligne
                              restore(e, key);
                            }}
                          >
                            {t("journal.restore")}
                          </button>
                        )}
                        {restored.has(key) && <span className="muted small">{restored.get(key)}</span>}
                      </span>
                    </summary>
                    <div className="journal-detail">
                      {/* La description dit ce qui a été fait : pas pour une action qui a échoué. */}
                      {KNOWN.includes(kind) && !e.error && <p>{t(`journal.desc.${kind}` as Key)}</p>}
                      {e.error && <p className="danger">{e.error}</p>}
                      {e.paths.length > 0 && (
                        <ul className="paths">
                          {e.paths.slice(0, 50).map((p, n) => (
                            <li key={n} className="path" title={p}>
                              {p}
                            </li>
                          ))}
                          {e.paths.length > 50 && <li className="muted">… +{e.paths.length - 50}</li>}
                        </ul>
                      )}
                      {e.output && <pre className="journal-output">{e.output}</pre>}
                      <p className="muted small path">{e.action}</p>
                    </div>
                  </details>
                </li>
              );
            })}
          </ul>
        </Card>
      ))}
    </>
  );
}
