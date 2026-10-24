import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { appVersion } from "../lib/version";
import { useT } from "../i18n/useT";
import { installUpdate, isInstalled } from "../lib/api";
import { DOWNLOAD_PAGE, findUpdate, updateCheckEnabled, type Update } from "../lib/update";

/**
 * Bouton de mise à jour. Version installée : télécharge, vérifie et lance l'installateur en un clic (Kleaner se ferme).
 * Version portable, ou si l'installation automatique échoue : la page de téléchargement.
 */
export function UpdateAction({ update }: { update: Update }) {
  const { t } = useT();
  const [installed, setInstalled] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    isInstalled().then(setInstalled, () => {});
  }, []);

  if (!installed || failed) {
    return (
      <>
        {failed && <span className="muted small">{t("update.installFailed")}</span>}
        <button className="btn btn-primary" onClick={() => openUrl(DOWNLOAD_PAGE)}>
          {t("update.download")} →
        </button>
      </>
    );
  }
  return (
    <button
      className="btn btn-primary"
      disabled={busy}
      onClick={() => {
        setBusy(true);
        installUpdate(update.version).catch(() => {
          setBusy(false);
          setFailed(true);
        });
      }}
    >
      {busy ? t("update.installing") : `${t("update.install")} ${update.version}`}
    </button>
  );
}

/** Bandeau « nouvelle version », seulement si l'utilisateur a activé la vérification. */
export function UpdateBanner() {
  const { t } = useT();
  const [update, setUpdate] = useState<Update | null>(null);

  useEffect(() => {
    if (!updateCheckEnabled()) return;
    appVersion()
      .then(findUpdate)
      .then(setUpdate, () => {});
  }, []);

  if (!update) return null;
  return (
    <div className="card cta">
      <strong>
        {t("update.available")} {update.version}
      </strong>
      <UpdateAction update={update} />
    </div>
  );
}
