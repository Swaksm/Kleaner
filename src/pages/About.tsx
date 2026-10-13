import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { useT, type Key } from "../i18n/useT";

const REPO = "https://github.com/Swaksm/Kleaner";
const AUTHOR = "https://github.com/Swaksm";

const DOES: Key[] = ["about.does.scan", "about.does.clean", "about.does.explain", "about.does.trash", "about.does.log"];
const NEVER: Key[] = ["about.never.confirm", "about.never.sessions", "about.never.registry", "about.never.network", "about.never.admin"];

export function About() {
  const { t } = useT();
  const [version, setVersion] = useState("");
  useEffect(() => {
    getVersion().then(setVersion, () => {});
  }, []);

  return (
    <>
      <Card>
        <div className="row stats">
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
        <div className="row">
          <button className="btn btn-primary" onClick={() => openUrl(REPO)}>
            {t("about.github")}
          </button>
          <button className="btn" onClick={() => openUrl(`${REPO}/issues`)}>
            {t("about.issue")}
          </button>
          <button className="btn" onClick={() => openUrl(`${REPO}/releases`)}>
            {t("about.releases")}
          </button>
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
