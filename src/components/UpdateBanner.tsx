import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { useT } from "../i18n/useT";
import { findUpdate, updateCheckEnabled, type Update } from "../lib/update";

/** Bandeau « nouvelle version », seulement si l'utilisateur a activé la vérification. */
export function UpdateBanner() {
  const { t } = useT();
  const [update, setUpdate] = useState<Update | null>(null);

  useEffect(() => {
    if (!updateCheckEnabled()) return;
    getVersion()
      .then(findUpdate)
      .then(setUpdate, () => {});
  }, []);

  if (!update) return null;
  return (
    <div className="card cta">
      <strong>
        {t("update.available")} {update.version}
      </strong>
      <button className="btn btn-primary" onClick={() => openUrl(update.url)}>
        {t("update.download")} →
      </button>
    </div>
  );
}
