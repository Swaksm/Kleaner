import { useEffect, useRef, useState } from "react";
import { useT } from "../i18n/useT";
import { revealInExplorer, type ScanResult } from "../lib/api";
import { formatBytes } from "../lib/format";
import { childrenOf, norm, squarify } from "../lib/treemap";

// Hauteur fixe ; la largeur suit le conteneur (pas d’étirement en plein écran).
const H = 320;
const OTHER = "\u0000other";

/** Carte du disque : rectangles proportionnels aux dossiers ; clic = entrer dans le dossier. */
export function DiskMap({ scan }: { scan: ScanResult }) {
  const { t, lang } = useT();
  const [chosen, setPath] = useState<string[]>([]);
  const box = useRef<HTMLDivElement>(null);
  const [W, setW] = useState(900);
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    setW(Math.max(200, el.clientWidth));
    const ro = new ResizeObserver(([e]) => setW(Math.max(200, Math.round(e.contentRect.width))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const roots = scan.roots.map((r) => scan.tree.find((i) => norm(i.path) === norm(r))).filter((i) => !!i);
  // Un seul disque : on part directement de son contenu.
  const path = chosen.length === 0 && roots.length === 1 ? [roots[0].path] : chosen;
  const current = path[path.length - 1];
  const node = current ? scan.tree.find((i) => i.path === current) : undefined;
  const children = current ? childrenOf(scan.tree, current) : roots;
  const total = node ? node.size : children.reduce((s, c) => s + c.size, 0);
  const rest = Math.max(0, total - children.reduce((s, c) => s + c.size, 0));
  const items = [...children, ...(rest > total * 0.005 ? [{ path: OTHER, size: rest, modified: 0 }] : [])];
  const tiles = squarify(items, (i) => i.size, { x: 0, y: 0, w: W, h: H });
  const name = (p: string) => p.slice(norm(p).lastIndexOf("\\") + 1) || p;

  return (
    <div className="diskmap" ref={box}>
      <nav className="row crumbs" aria-label={t("map.path")}>
        <button className="link" onClick={() => setPath([])}>
          {t("map.all")}
        </button>
        {path.map((p, i) => (
          <span key={p}>
            ›{" "}
            <button className="link" onClick={() => setPath(path.slice(0, i + 1))}>
              {name(p)}
            </button>
          </span>
        ))}
        {current && (
          <button className="btn btn-small" onClick={() => revealInExplorer(current)}>
            {t("analyse.open")}
          </button>
        )}
      </nav>
      <svg viewBox={`0 0 ${W} ${H}`} width={W} height={H} className="diskmap-svg" role="img" aria-label={t("map.title")}>
        {tiles.map((tile) => {
          const other = tile.item.path === OTHER;
          const label = other ? t("map.other") : name(tile.item.path);
          const big = tile.w > 90 && tile.h > 34;
          const canEnter = !other && childrenOf(scan.tree, tile.item.path).length > 0;
          return (
            <g
              key={tile.item.path}
              className={other ? "tile-other" : canEnter ? "tile-dir enter" : "tile-dir"}
              onClick={() => canEnter && setPath([...path, tile.item.path])}
            >
              <rect x={tile.x + 1} y={tile.y + 1} width={Math.max(0, tile.w - 2)} height={Math.max(0, tile.h - 2)} rx={4} />
              <title>{`${other ? label : tile.item.path} — ${formatBytes(tile.item.size, lang)}`}</title>
              {big && (
                <>
                  <text x={tile.x + 8} y={tile.y + 18} className="tile-label">
                    {label.length > tile.w / 8 ? `${label.slice(0, Math.floor(tile.w / 8) - 1)}…` : label}
                  </text>
                  <text x={tile.x + 8} y={tile.y + 33} className="tile-size">
                    {formatBytes(tile.item.size, lang)}
                  </text>
                </>
              )}
            </g>
          );
        })}
      </svg>
      <p className="muted small">{t("map.hint")}</p>
    </div>
  );
}
