import { useState } from "react";
import { useT } from "../i18n/useT";
import { relaunchAdmin } from "../lib/api";
import { StatusPill } from "./StatusPill";

/** Mode courant + bouton pour relancer Kleaner en admin (fenêtre UAC de Windows). */
export function AdminBanner({ elevated }: { elevated: boolean }) {
  const { t } = useT();
  const [error, setError] = useState<string | null>(null);

  if (elevated) return <StatusPill status="ok">{t("admin.on")}</StatusPill>;
  return (
    <div className="row">
      <StatusPill status="neutral">{t("admin.off")}</StatusPill>
      <span className="muted">{t("admin.why")}</span>
      <button className="btn" onClick={() => relaunchAdmin().catch((e) => setError(String(e)))}>
        {t("admin.button")}
      </button>
      {error && <StatusPill status="danger">{error}</StatusPill>}
    </div>
  );
}
