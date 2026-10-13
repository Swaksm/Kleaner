import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { previewMedia } from "../lib/api";

/**
 * Miniature d'une photo ou d'une vidéo, chargée seulement quand la tuile devient visible
 * (le Rust n'autorise la lecture du fichier qu'à ce moment-là).
 * Vidéo : une image prise à 1 s (métadonnées seulement, pas de lecture).
 */
// shortcut: la photo est décodée en taille réelle pour la miniature ; générer des miniatures côté Rust si la grille rame.
export function Thumb({ path, alt, video = false }: { path: string; alt: string; video?: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const io = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return;
        io.disconnect();
        previewMedia(path).then(
          (p) => setSrc(convertFileSrc(p)),
          () => setFailed(true),
        );
      },
      { rootMargin: "200px" },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [path]);

  const fallback = (
    <span className="thumb-icon" aria-hidden>
      {video ? "▶" : failed ? "🖼" : ""}
    </span>
  );

  return (
    <div ref={ref} className="thumb">
      {src && !failed ? (
        video ? (
          <>
            <video src={`${src}#t=1`} preload="metadata" muted playsInline onError={() => setFailed(true)} />
            <span className="thumb-play" aria-hidden>
              ▶
            </span>
          </>
        ) : (
          <img src={src} alt={alt} loading="lazy" decoding="async" onError={() => setFailed(true)} />
        )
      ) : (
        fallback
      )}
    </div>
  );
}
