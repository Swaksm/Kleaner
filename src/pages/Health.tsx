import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { StatusPill } from "../components/StatusPill";
import { useT, type Key } from "../i18n/useT";
import { healthDism, healthReport, openSystemPage, type HealthItem } from "../lib/api";

const GROUPS = ["system", "disk", "security"] as const;
const PILL = { ok: "ok", warn: "warn", bad: "danger", info: "neutral", unknown: "neutral" } as const;
const ORDER = ["bad", "warn", "unknown", "info", "ok"];

/** Bilan de santé et de sécurité, en lecture seule : chaque alerte renvoie vers l'écran de Windows qui la règle. */
export function Health() {
  const { t, tx } = useT();
  const [items, setItems] = useState<HealthItem[] | null>(null);
  const [elevated, setElevated] = useState(false);
  const [dism, setDism] = useState<HealthItem | null>(null);
  const [dismBusy, setDismBusy] = useState(false);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  function load() {
    setItems(null);
    healthReport().then(
      (r) => {
        setItems(r.items);
        setElevated(r.elevated);
      },
      (e) => setMessage({ status: "danger", text: String(e) }),
    );
  }

  useEffect(load, []);

  async function checkFiles() {
    setDismBusy(true);
    setMessage(null);
    try {
      setDism(await healthDism());
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    } finally {
      setDismBusy(false);
    }
  }

  const all = items ? [...items, ...(dism ? [dism] : [])] : [];
  const todo = all.filter((i) => i.status === "warn" || i.status === "bad").length;

  const row = (i: HealthItem, n: number) => (
    <li key={`${i.id}-${n}`}>
      <div className="program">
        <strong>{tx(i.title)}</strong>
        <span className="muted small">{tx(i.detail)}</span>
      </div>
      <StatusPill status={PILL[i.status]}>{t(`health.status.${i.status}` as Key)}</StatusPill>
      {i.fix ? (
        <button className="btn btn-small" onClick={() => openSystemPage(i.fix!).catch((e) => setMessage({ status: "danger", text: String(e) }))}>
          {t("health.open")}
        </button>
      ) : (
        <span />
      )}
    </li>
  );

  return (
    <>
      <Card>
        <p>{t("health.intro")}</p>
        <div className="row">
          <button className="btn btn-primary" disabled={!items} onClick={load}>
            {t("health.run")}
          </button>
          {!items && (
            <span className="loading" role="status">
              <span className="spinner" aria-hidden /> {t("health.running")}
            </span>
          )}
          {items && (
            <StatusPill status={todo > 0 ? "warn" : "ok"}>{todo > 0 ? `${todo} ${t("health.attention")}` : t("health.allGood")}</StatusPill>
          )}
        </div>
        {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      </Card>

      {items &&
        GROUPS.map((g) => {
          const list = all.filter((i) => i.group === g).sort((a, b) => ORDER.indexOf(a.status) - ORDER.indexOf(b.status));
          return (
            list.length > 0 && (
              <Card key={g} title={t(`health.group.${g}` as Key)}>
                <ul className="items grid-health">{list.map(row)}</ul>
                {g === "system" && (
                  <div className="row">
                    <button className="btn" disabled={!elevated || dismBusy} title={t("health.dismHint")} onClick={checkFiles}>
                      {t("health.dism")}
                    </button>
                    {dismBusy && (
                      <span className="loading" role="status">
                        <span className="spinner" aria-hidden />
                      </span>
                    )}
                    {!elevated && <span className="muted small">{t("health.dismHint")}</span>}
                  </div>
                )}
              </Card>
            )
          );
        })}
    </>
  );
}
