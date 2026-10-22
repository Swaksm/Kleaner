import { useState } from "react";
import { useT, type Key } from "../i18n/useT";

const STORAGE_KEY = "disclaimerAccepted";
const points: Key[] = [
  "disclaimer.does",
  "disclaimer.never",
  "disclaimer.network",
  "disclaimer.caches",
  "disclaimer.journal",
  "disclaimer.license",
];

export function isDisclaimerAccepted(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

/** Écran bloquant au premier lancement : rien n'est accessible avant acceptation. */
export function Disclaimer({ onAccept }: { onAccept: () => void }) {
  const { t } = useT();
  const [read, setRead] = useState(false);

  function accept() {
    try {
      localStorage.setItem(STORAGE_KEY, "1");
    } catch {}
    onAccept();
  }

  return (
    <div className="disclaimer">
      <div className="card disclaimer-card">
        <span className="brand-logo brand-logo-big" aria-hidden />
        <h1>{t("disclaimer.title")}</h1>
        <ul>
          {points.map((k) => (
            <li key={k}>{t(k)}</li>
          ))}
        </ul>
        <label className="check">
          <input type="checkbox" checked={read} onChange={(e) => setRead(e.target.checked)} />
          {t("disclaimer.read")}
        </label>
        <button className="btn btn-primary" disabled={!read} onClick={accept}>
          {t("disclaimer.accept")}
        </button>
      </div>
    </div>
  );
}
