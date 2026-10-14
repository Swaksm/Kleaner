import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { StatusPill } from "../components/StatusPill";
import { useT, type Key } from "../i18n/useT";
import { listStartup, setStartup, type StartupItem } from "../lib/api";
import { formatDate } from "../lib/format";

type View = { items: StartupItem[]; boots: { time: number; boot_ms: number }[] };

const secs = (ms: number) => `${(ms / 1000).toFixed(ms < 10_000 ? 1 : 0)} s`;

/** Programmes lancés à l'ouverture de session : on peut les désactiver et les réactiver, rien n'est supprimé. */
export function Startup() {
  const { t, lang } = useT();
  const [view, setView] = useState<View | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [asking, setAsking] = useState<StartupItem | null>(null);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  function load() {
    setView(null);
    listStartup().then(setView, (e) => setMessage({ status: "danger", text: String(e) }));
  }

  useEffect(load, []);

  async function change(item: StartupItem, enabled: boolean) {
    setAsking(null);
    setBusy(item.id);
    setMessage(null);
    try {
      await setStartup(item.id, enabled);
      setView((v) => v && { ...v, items: v.items.map((i) => (i.id === item.id ? { ...i, enabled } : i)) });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    } finally {
      setBusy(null);
    }
  }

  const items = view?.items.filter((i) => i.can_toggle) ?? [];
  const services = view?.items.filter((i) => !i.can_toggle) ?? [];
  const boots = view?.boots ?? [];
  const average = boots.length ? boots.reduce((s, b) => s + b.boot_ms, 0) / boots.length : 0;

  return (
    <>
      <Card>
        <p>{t("startup.intro")}</p>
        {view && (
          <div className="row">
            <span className="num big">{items.filter((i) => i.enabled).length}</span>
            <span className="muted">{t("startup.enabledCount")}</span>
            {boots.length > 0 ? (
              <span className="muted">
                · {t("startup.lastBoot")} <span className="num">{secs(boots[0].boot_ms)}</span> ({formatDate(boots[0].time, lang)}) ·{" "}
                {t("startup.average")} <span className="num">{secs(average)}</span>
              </span>
            ) : (
              <span className="muted small">{t("startup.noBoots")}</span>
            )}
          </div>
        )}
        {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
      </Card>

      <Card>
        {!view ? (
          <span className="loading" role="status">
            <span className="spinner" aria-hidden /> {t("startup.reading")}
          </span>
        ) : (
          <ul className="items grid-startup">
            {items.map((i) => (
              <li key={i.id} className={i.enabled ? undefined : "muted"}>
                <div className="program">
                  <strong>{i.name}</strong>
                  <span className="muted path small" title={i.command}>
                    {i.command}
                  </span>
                </div>
                <span className="muted small">
                  {t(`startup.source.${i.source}` as Key)}
                  {i.machine && <span className="block">{t("startup.allUsers")}</span>}
                </span>
                <span className="tags">
                  {i.slow_ms > 0 && (
                    <span title={t("startup.slowHint")}>
                      <StatusPill status="warn">+{secs(i.slow_ms)}</StatusPill>
                    </span>
                  )}
                  {i.keep && (
                    <span title={t("startup.keepHint")}>
                      <StatusPill status="neutral">{t("startup.keep")}</StatusPill>
                    </span>
                  )}
                </span>
                {i.enabled ? (
                  <button className="btn btn-small" disabled={busy === i.id} onClick={() => (i.keep ? setAsking(i) : change(i, false))}>
                    {t("startup.disable")}
                  </button>
                ) : (
                  <button className="btn btn-small btn-primary" disabled={busy === i.id} onClick={() => change(i, true)}>
                    {t("startup.enable")}
                  </button>
                )}
              </li>
            ))}
          </ul>
        )}
      </Card>

      {services.length > 0 && (
        <Card title={t("startup.services")}>
          <p className="muted small">{t("startup.servicesHint")}</p>
          <ul className="items grid-startup">
            {services.map((i) => (
              <li key={i.id}>
                <div className="program">
                  <strong>{i.name}</strong>
                  <span className="muted path small" title={i.command}>
                    {i.command}
                  </span>
                </div>
                <span className="muted small">{t("startup.source.service")}</span>
                <span className="tags">
                  {i.keep && <StatusPill status="neutral">{t("startup.keep")}</StatusPill>}
                </span>
                <span className="muted small">{i.enabled ? t("startup.running") : t("startup.stopped")}</span>
              </li>
            ))}
          </ul>
        </Card>
      )}

      <ConfirmDialog
        open={asking !== null}
        title={t("startup.keepConfirmTitle")}
        confirmLabel={t("startup.disable")}
        cancelLabel={t("common.cancel")}
        danger
        onConfirm={() => asking && change(asking, false)}
        onCancel={() => setAsking(null)}
      >
        <p>
          <strong>{asking?.name}</strong>
        </p>
        <p>{t("startup.keepConfirm")}</p>
      </ConfirmDialog>
    </>
  );
}
