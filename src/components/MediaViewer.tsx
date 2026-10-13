import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { useT } from "../i18n/useT";
import { openMedia, previewMedia, revealInExplorer, type Item } from "../lib/api";
import { formatBytes, formatDay } from "../lib/format";
import { baseName } from "../lib/sort";
import { StatusPill } from "./StatusPill";

/** Clic sur le fond assombri (hors du cadre de la fenêtre). */
export function outside(e: React.MouseEvent, d: HTMLElement | null): boolean {
  if (!d || e.target !== d) return false;
  const r = d.getBoundingClientRect();
  return e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom;
}

/**
 * Aperçu d'une photo ou lecture d'une vidéo dans Kleaner.
 * Le Rust n'autorise la lecture que du fichier affiché ; formats non lisibles → lecteur par défaut.
 * ← → : média précédent / suivant. Échap : fermer.
 */
export function MediaViewer({
  items,
  index,
  kind,
  onIndex,
  onClose,
}: {
  items: Item[];
  index: number;
  kind: "videos" | "images";
  onIndex: (i: number) => void;
  onClose: () => void;
}) {
  const { t, lang } = useT();
  const ref = useRef<HTMLDialogElement>(null);
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const [zoom, setZoom] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const item = items[index];

  useEffect(() => {
    ref.current?.showModal();
  }, []);

  useEffect(() => {
    setSrc(null);
    setFailed(false);
    setZoom(false);
    setError(null);
    previewMedia(item.path).then(
      (path) => setSrc(convertFileSrc(path)),
      (e) => setError(String(e)),
    );
  }, [item.path]);

  function onKey(e: React.KeyboardEvent) {
    if (e.key === "ArrowRight" && index < items.length - 1) onIndex(index + 1);
    if (e.key === "ArrowLeft" && index > 0) onIndex(index - 1);
  }

  const open = () => openMedia(item.path).catch((e) => setError(String(e)));

  return (
    <dialog
      ref={ref}
      className="dialog viewer"
      onCancel={onClose}
      onKeyDown={onKey}
      onClick={(e) => outside(e, ref.current) && onClose()}
    >
      <div className="viewer-head">
        <div className="media-text">
          <strong>{baseName(item.path)}</strong>
          <span className="muted path">{item.path}</span>
        </div>
        <span className="muted">
          {formatDay(item.modified, lang)} · <span className="num">{formatBytes(item.size, lang)}</span> · {index + 1}/
          {items.length}
        </span>
      </div>

      <div className={zoom ? "viewer-stage zoom" : "viewer-stage"}>
        {src && !failed && kind === "images" && (
          <img
            src={src}
            alt={baseName(item.path)}
            onClick={() => setZoom(!zoom)}
            onError={() => setFailed(true)}
            title={t(zoom ? "viewer.fit" : "viewer.zoom")}
          />
        )}
        {src && !failed && kind === "videos" && (
          <video key={src} src={src} controls autoPlay onError={() => setFailed(true)} />
        )}
        {(failed || error) && (
          <div className="viewer-fallback">
            <p>{error ?? t("viewer.unsupported")}</p>
            <button className="btn btn-primary" onClick={open}>
              {t("viewer.openDefault")}
            </button>
          </div>
        )}
      </div>

      <div className="dialog-actions">
        <button className="btn" disabled={index === 0} onClick={() => onIndex(index - 1)}>
          ← {t("viewer.prev")}
        </button>
        <button className="btn" disabled={index >= items.length - 1} onClick={() => onIndex(index + 1)}>
          {t("viewer.next")} →
        </button>
        <span className="spacer" />
        <button className="btn" onClick={() => revealInExplorer(item.path)}>
          {t("analyse.open")}
        </button>
        <button className="btn" onClick={open}>
          {t("viewer.openDefault")}
        </button>
        <button className="btn btn-primary" onClick={onClose}>
          {t("report.close")}
        </button>
      </div>
      {error && !failed && <StatusPill status="danger">{error}</StatusPill>}
    </dialog>
  );
}
