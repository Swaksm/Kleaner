import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { BugReportButton } from "../components/BugReportButton";
import { Card } from "../components/Card";
import { findUpdate } from "../lib/update";
import { appVersion } from "../lib/version";
import { useT, type Key } from "../i18n/useT";

const REPO = "https://github.com/Swaksm/Kleaner";
const AUTHOR = "https://github.com/Swaksm";
const WINAPP2 = "https://github.com/MoscaDotTo/Winapp2";

const DOES: Key[] = ["about.does.scan", "about.does.clean", "about.does.explain", "about.does.trash", "about.does.log"];
const NEVER: Key[] = ["about.never.confirm", "about.never.sessions", "about.never.registry", "about.never.network", "about.never.admin"];

export function About() {
  const { t } = useT();
  const [version, setVersion] = useState("");
  const [check, setCheck] = useState<string | null>(null);
  const [updateUrl, setUpdateUrl] = useState<string | null>(null);

  async function checkNow() {
    setCheck(t("update.checking"));
    try {
      const u = await findUpdate(version);
      setUpdateUrl(u?.url ?? null);
      setCheck(u ? `${t("update.available")} ${u.version}` : t("update.upToDate"));
    } catch (e) {
      setCheck(String(e));
    }
  }
  useEffect(() => {
    appVersion().then(setVersion, () => {});
  }, []);

  return (
    <>
      <Card>
        <div className="row stats">
          <span className="brand-logo brand-logo-big" aria-hidden />
          <span className="big">Kleaner</span>
          {version && <span className="num muted">v{version}</span>}
        </div>
        <p>{t("about.tagline")}</p>
        <p>
          {t("about.by")}{" "}
          <button className="link" onClick={() => openUrl(AUTHOR)}>
            swaks
          </button>
        </p>
        <p className="muted small">
          {t("about.winapp2")}{" "}
          <button className="link" onClick={() => openUrl(WINAPP2)}>
            Winapp2
          </button>{" "}
          (CC BY-SA 4.0)
        </p>
        <div className="row">
          <button className="btn btn-primary" onClick={() => openUrl(REPO)}>
            {t("about.github")}
          </button>
          <button className="btn" onClick={() => openUrl(`${REPO}/releases`)}>
            {t("about.releases")}
          </button>
          <BugReportButton />
        </div>
        <div className="row">
          <button className="btn" disabled={!version} onClick={checkNow}>
            {t("update.checkNow")}
          </button>
          {check && <span className="muted">{check}</span>}
          {updateUrl && (
            <button className="btn btn-primary" onClick={() => openUrl(updateUrl)}>
              {t("update.download")} →
            </button>
          )}
        </div>
      </Card>

      <Card title={t("about.doesTitle")}>
        <ul className="plain">
          {DOES.map((k) => (
            <li key={k}>✓ {t(k)}</li>
          ))}
        </ul>
      </Card>

      <Card title={t("about.neverTitle")}>
        <ul className="plain">
          {NEVER.map((k) => (
            <li key={k}>✗ {t(k)}</li>
          ))}
        </ul>
      </Card>

      <Card title={t("about.disclaimerTitle")}>
        <p>{t("disclaimer.license")}</p>
        <p className="muted">{t("about.verdictsWarning")}</p>
        <p className="muted">{t("about.linksNote")}</p>
      </Card>
    </>
  );
}
