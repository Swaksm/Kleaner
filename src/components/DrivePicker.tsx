import { useT, type Key } from "../i18n/useT";
import type { DriveInfo } from "../lib/api";
import { formatBytes } from "../lib/format";

/** Cases à cocher des disques à analyser, avec type et espace libre. */
export function DrivePicker({
  drives,
  selected,
  onChange,
}: {
  drives: DriveInfo[];
  selected: Set<string>;
  onChange: (next: Set<string>) => void;
}) {
  const { t, lang } = useT();
  function toggle(root: string) {
    const next = new Set(selected);
    if (next.has(root)) next.delete(root);
    else next.add(root);
    onChange(next);
  }
  return (
    <div className="drives" role="group" aria-label={t("drives.title")}>
      {drives.map((d) => (
        <label key={d.root} className="check drive">
          <input type="checkbox" checked={selected.has(d.root)} onChange={() => toggle(d.root)} />
          <strong>{d.root}</strong>
          <span className="muted small">{t(`drives.${d.kind}` as Key)}</span>
          <span className="muted small">
            {formatBytes(d.free, lang)} {t("analyse.freeOf")} {formatBytes(d.total, lang)}
          </span>
        </label>
      ))}
    </div>
  );
}
