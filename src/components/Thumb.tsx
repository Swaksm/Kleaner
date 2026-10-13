import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { previewMedia } from "../lib/api";

/**
 * Miniature d'une photo, chargée seulement quand la tuile devient visible
 * (le Rust n'autorise la lecture du fichier qu'à ce moment-là).
 */
export function Thumb({ path, alt }: { path: string; alt: string }) {
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

  return (
    <div ref={ref} className="thumb">
      {src && !failed ? (
        <img src={src} alt={alt} loading="lazy" decoding="async" onError={() => setFailed(true)} />
      ) : (
        <span className="thumb-icon" aria-hidden>
          {failed ? "🖼" : ""}
        </span>
      )}
    </div>
  );
}
