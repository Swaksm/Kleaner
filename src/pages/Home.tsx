import { useEffect, useState } from "react";
import { Card } from "../components/Card";
import { SizeBar } from "../components/SizeBar";
import { useT, type Key } from "../i18n/useT";
import { findLeftovers, listDrives, onedriveRoots, type DriveInfo } from "../lib/api";
import { formatBytes } from "../lib/format";
import { useScanResult } from "../lib/scanStore";
import { useRecoverable } from "../lib/useRecoverable";

type Target = "analyse" | "cleanup" | "programs" | "projects" | "duplicates" | "onedrive";
const MONTH = 30 * 86_400;

/** Accueil : le PC en un coup d'œil et les quelques actions qui libèrent le plus de place. */
export function Home({ onScan, go, onQuickClean }: { onScan: () => void; go: (page: Target) => void; onQuickClean: () => void }) {
  const { t, lang } = useT();
  const scan = useScanResult();
  const recoverable = useRecoverable(scan);
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [leftovers, setLeftovers] = useState<number | null>(null);
  const [onedrive, setOnedrive] = useState(false);

  useEffect(() => {
    listDrives().then((d) => setDrives(d.filter((x) => x.kind === "fixed")), () => {});
    onedriveRoots().then((r) => setOnedrive(r.length > 0), () => {});
  }, []);

  useEffect(() => {
    if (!scan) return;
    findLeftovers().then((l) => setLeftovers(l.filter((x) => x.trashable).reduce((s, x) => s + x.size, 0)), () => {});
  }, [scan?.duration_ms]);

  const oldProjects = scan?.artifacts.filter((a) => a.project_modified < Date.now() / 1000 - 3 * MONTH).reduce((s, a) => s + a.size, 0) ?? 0;

  // Recommandations, de la plus sûre à la plus personnelle ; masquées si rien à gagner.
  const recs: { key: Key; detail: Key; size: number | null; target: Target }[] = [
    { key: "home.rec.cleanup", detail: "home.rec.cleanupDetail", size: recoverable, target: "cleanup" },
    { key: "home.rec.leftovers", detail: "home.rec.leftoversDetail", size: leftovers, target: "programs" },
    ...(oldProjects > 0 ? [{ key: "home.rec.projects" as Key, detail: "home.rec.projectsDetail" as Key, size: oldProjects, target: "projects" as Target }] : []),
    { key: "home.rec.bigFiles", detail: "home.rec.bigFilesDetail", size: null, target: "analyse" },
    { key: "home.rec.duplicates", detail: "home.rec.duplicatesDetail", size: null, target: "duplicates" },
    ...(onedrive ? [{ key: "home.rec.onedrive" as Key, detail: "home.rec.onedriveDetail" as Key, size: null, target: "onedrive" as Target }] : []),
  ];

  return (
    <>
      <Card title={t("home.drives")}>
        {drives.map((d) => (
          <SizeBar
            key={d.root}
            label={`${d.root} · ${formatBytes(d.free, lang)} ${t("analyse.freeOf")} ${formatBytes(d.total, lang)}`}
            value={`${Math.round(((d.total - d.free) / Math.max(1, d.total)) * 100)} % ${t("analyse.used")}`}
            ratio={(d.total - d.free) / Math.max(1, d.total)}
          />
        ))}
      </Card>

      {!scan ? (
        <div className="card hero">
          <p>{t("home.intro")}</p>
          <button className="btn btn-primary btn-big" onClick={onScan}>
            {t("analyse.start")}
          </button>
        </div>
      ) : (
        <Card title={t("home.recs")}>
          {!!recoverable && (
            <div className="quick-clean">
              <button className="btn btn-primary btn-big" onClick={onQuickClean}>
                {t("home.quickClean")} ({formatBytes(recoverable, lang)})
              </button>
              <p className="muted small">{t("home.quickCleanNote")}</p>
            </div>
          )}
          <ul className="recs">
            {recs
              .filter((r) => r.size === null || r.size > 0)
              .map((r) => (
                <li key={r.key}>
                  <div>
                    <strong>{t(r.key)}</strong>
                    <p className="muted small">{t(r.detail)}</p>
                  </div>
                  {r.size !== null && <span className="num big">{formatBytes(r.size, lang)}</span>}
                  <button className="btn" onClick={() => go(r.target)}>
                    {t("home.see")} →
                  </button>
                </li>
              ))}
          </ul>
          <button className="btn" onClick={onScan}>
            {t("analyse.restart")}
          </button>
        </Card>
      )}
    </>
  );
}
