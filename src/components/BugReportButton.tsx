import { openUrl } from "@tauri-apps/plugin-opener";
import { useState } from "react";
import { useT } from "../i18n/useT";
import { bugReport, revealInExplorer } from "../lib/api";
import { StatusPill } from "./StatusPill";

const NEW_ISSUE = "https://github.com/Swaksm/Kleaner/issues/new";

/**
 * Prépare un rapport (fichier local, sans chemins personnels), le montre dans l'Explorateur
 * et ouvre un ticket GitHub vide dans le navigateur. Rien n'est envoyé par Kleaner.
 */
export function BugReportButton() {
  const { t } = useT();
  const [message, setMessage] = useState<{ status: "ok" | "danger"; text: string } | null>(null);

  async function report() {
    try {
      const file = await bugReport();
      await revealInExplorer(file);
      // Aucune donnée dans l'URL : seulement un titre et un modèle ; le fichier se joint à la main.
      const body = encodeURIComponent(t("bug.template").split("{n}").join("\n"));
      await openUrl(`${NEW_ISSUE}?title=${encodeURIComponent("[Bug] ")}&body=${body}`);
      setMessage({ status: "ok", text: t("bug.ready") });
    } catch (e) {
      setMessage({ status: "danger", text: String(e) });
    }
  }

  return (
    <>
      <button className="btn" onClick={report}>
        {t("bug.button")}
      </button>
      {message && <StatusPill status={message.status}>{message.text}</StatusPill>}
    </>
  );
}
