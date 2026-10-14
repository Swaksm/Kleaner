import { openUrl } from "@tauri-apps/plugin-opener";
import { Fragment, useEffect, useState } from "react";
import { Card } from "../components/Card";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { StatusPill } from "../components/StatusPill";
import { useT } from "../i18n/useT";
import {
  appUpdates,
  openWindowsUpdate,
  systemInfo,
  upgradeApps,
  windowsUpdates,
  type AppUpdate,
  type Gpu,
  type SystemInfo,
  type WinUpdateStatus,
} from "../lib/api";
import { formatBytes } from "../lib/format";

const DAY = 86_400_000;

/** « 20250326 » → date. */
const ymd = (s: string) => (/^\d{8}$/.test(s) ? new Date(+s.slice(0, 4), +s.slice(4, 6) - 1, +s.slice(6, 8)) : null);

/** Page officielle de pilotes du fabricant (jamais de téléchargement automatique). */
function vendorPage(g: Gpu): string | null {
  const v = `${g.vendor} ${g.name}`.toLowerCase();
  if (v.includes("nvidia")) return "https://www.nvidia.com/en-us/drivers/";
  if (v.includes("amd") || v.includes("radeon") || v.includes("advanced micro")) return "https://www.amd.com/en/support/download/drivers.html";
  if (v.includes("intel")) return "https://www.intel.com/content/www/us/en/support/detect.html";
  return null;
}

export function Updates() {
  const { t, lang } = useT();
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [win, setWin] = useState<WinUpdateStatus | null>(null);
  const [winBusy, setWinBusy] = useState(false);
  const [apps, setApps] = useState<AppUpdate[] | null>(null);
  const [appsBusy, setAppsBusy] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirm, setConfirm] = useState(false);
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);
  const day = (d: Date | null) => (d ? d.toLocaleDateString(lang) : "—");

  useEffect(() => {
    systemInfo().then(setInfo, (e) => setMessage({ status: "danger", text: String(e) }));
  }, []);

  async function checkWindows() {
    setWinBusy(true);
    try {
      setWin(await windowsUpdates());
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    } finally {
      setWinBusy(false);
    }
  }

  async function checkApps() {
    setAppsBusy(true);
    try {
      setApps(await appUpdates());
      setSelected(new Set());
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    } finally {
      setAppsBusy(false);
    }
  }

  async function upgrade() {
    setConfirm(false);
    setAppsBusy(true);
    const ids = [...selected];
    try {
      const [ok] = await upgradeApps(ids);
      setMessage({ status: ok === ids.length ? "ok" : "danger", text: `${ok}/${ids.length} ${t("updates.appsDone")}` });
      setApps(await appUpdates());
      setSelected(new Set());
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    } finally {
      setAppsBusy(false);
    }
  }

  function toggle(id: string) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelected(next);
  }

  const end = info?.support_end ? new Date(info.support_end) : null;
  const ended = end !== null && end.getTime() < Date.now();
  const soon = end !== null && !ended && end.getTime() - Date.now() < 120 * DAY;
  const days = info ? Math.floor(info.uptime / 86_400) : 0;

  return (
    <>
      <p className="muted">{t("updates.intro")}</p>
      {message && <StatusPill status={message.status}>{message.text}</StatusPill>}

      <Card title={t("updates.windows")}>
        {info && (
          <div className="row">
            <strong>{info.os_name}</strong>
            <span className="muted">
              {info.os_release} · build {info.os_build}
            </span>
            {end && (
              <StatusPill status={ended ? "danger" : soon ? "warn" : "ok"}>
                {ended ? t("updates.supportEnded") : t("updates.supportUntil")} {day(end)}
              </StatusPill>
            )}
          </div>
        )}
        {ended && <p className="danger">{t("updates.supportEndedHelp")}</p>}
        <div className="row">
          <button className="btn btn-primary" disabled={winBusy} onClick={checkWindows}>
            {t("updates.checkWindows")}
          </button>
          <button className="btn" onClick={() => openWindowsUpdate().catch((e) => setMessage({ status: "danger", text: String(e) }))}>
            {t("updates.openWindowsUpdate")}
          </button>
          {winBusy && (
            <span className="loading" role="status">
              <span className="spinner" aria-hidden /> {t("updates.searching")}
            </span>
          )}
        </div>
        {win && (
          <>
            <div className="row">
              {win.reboot_required && <StatusPill status="warn">{t("updates.rebootRequired")}</StatusPill>}
              <span className="muted small">
                {t("updates.lastCheck")} {win.last_check || "—"}
              </span>
            </div>
            {win.updates.length === 0 ? (
              <p className="ok">{t("updates.windowsUpToDate")}</p>
            ) : (
              <ul className="plain">
                {win.updates.map((u) => (
                  <li key={u.title}>
                    <StatusPill status="neutral">{u.kind === "Driver" ? t("updates.driver") : t("updates.system")}</StatusPill> {u.title}
                    {u.size > 0 && <span className="muted small"> · {formatBytes(u.size, lang)}</span>}
                  </li>
                ))}
              </ul>
            )}
          </>
        )}
      </Card>

      <Card title={t("updates.apps")}>
        <p className="muted small">{t("updates.appsIntro")}</p>
        <div className="row">
          <button className="btn btn-primary" disabled={appsBusy} onClick={checkApps}>
            {t("updates.checkApps")}
          </button>
          {apps && apps.length > 0 && (
            <>
              <button className="btn" onClick={() => setSelected(new Set(apps.map((a) => a.id)))}>
                {t("media.selectVisible")}
              </button>
              <button className="btn" disabled={selected.size === 0 || appsBusy} onClick={() => setConfirm(true)}>
                {t("updates.updateSelected")} ({selected.size})
              </button>
            </>
          )}
          {appsBusy && (
            <span className="loading" role="status">
              <span className="spinner" aria-hidden /> {t("updates.searching")}
            </span>
          )}
        </div>
        {apps && apps.length === 0 && <p className="ok">{t("updates.appsUpToDate")}</p>}
        {apps && apps.length > 0 && (
          <ul className="items grid-apps">
            {apps.map((a) => (
              <li key={a.id}>
                <label className="check path" title={a.id}>
                  <input type="checkbox" checked={selected.has(a.id)} onChange={() => toggle(a.id)} />
                  <span>{a.name}</span>
                </label>
                <span className="muted small">{a.version}</span>
                <span className="small">→ {a.available}</span>
              </li>
            ))}
          </ul>
        )}
      </Card>

      <Card title={t("updates.drivers")}>
        <p className="muted small">{t("updates.driversIntro")}</p>
        {info?.gpus.map((g) => {
          const date = ymd(g.driver_date);
          const old = date !== null && Date.now() - date.getTime() > 365 * DAY;
          const page = vendorPage(g);
          return (
            <div key={g.name} className="row">
              <strong>{g.name}</strong>
              <span className="muted small">
                {g.driver_version} · {day(date)}
              </span>
              <StatusPill status={old ? "warn" : "ok"}>{old ? t("updates.driverOld") : t("updates.driverRecent")}</StatusPill>
              {page && (
                <button className="btn btn-small" onClick={() => openUrl(page)}>
                  {t("updates.vendorPage")}
                </button>
              )}
            </div>
          );
        })}
      </Card>

      <Card title={t("updates.machine")}>
        {info && (
          <dl className="cleaner-info">
            <dt>{t("updates.model")}</dt>
            <dd>
              {info.manufacturer} {info.model}
            </dd>
            <dt>{t("updates.memory")}</dt>
            <dd>{formatBytes(info.memory, lang)}</dd>
            <dt>BIOS</dt>
            <dd>
              {info.bios_version} · {day(ymd(info.bios_date))}
            </dd>
            <dt>{t("updates.uptime")}</dt>
            <dd>
              {days} {t("updates.days")}
              {days >= 7 && <span className="warn"> · {t("updates.restartHint")}</span>}
            </dd>
            {info.disks.map((d) => (
              <Fragment key={d.name}>
                <dt>{d.name}</dt>
                <dd>
                  <StatusPill status={d.health === "Healthy" ? "ok" : d.health === "Warning" ? "warn" : "danger"}>
                    {d.health === "Healthy" ? t("updates.diskHealthy") : d.health || "?"}
                  </StatusPill>{" "}
                  <span className="muted small">
                    {d.media} · {formatBytes(d.size, lang)}
                  </span>
                </dd>
              </Fragment>
            ))}
          </dl>
        )}
      </Card>

      <ConfirmDialog
        open={confirm}
        title={t("updates.confirmTitle")}
        confirmLabel={t("updates.updateSelected")}
        cancelLabel={t("common.cancel")}
        onConfirm={upgrade}
        onCancel={() => setConfirm(false)}
      >
        <p>{t("updates.confirmText")}</p>
        <ul className="paths">
          {[...selected].map((id) => (
            <li key={id}>{id}</li>
          ))}
        </ul>
      </ConfirmDialog>
    </>
  );
}
