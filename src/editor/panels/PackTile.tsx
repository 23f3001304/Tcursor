import { useEffect, useState, type CSSProperties, type ReactNode, type RefObject } from "react";
import { motion, useReducedMotion } from "motion/react";
import { alphaBounds, fitBox, type Box } from "./glyphFit";

const PRESS = { type: "spring" as const, stiffness: 500, damping: 30 };

const PLATE = 34,
  FIT_W = 20,
  FIT_H = 24;

const UNFITTED: CSSProperties = { width: 22, height: 22, left: (PLATE - 22) / 2, top: (PLATE - 22) / 2 };

function useGlyphFit(src: string): CSSProperties {
  const [fit, setFit] = useState<CSSProperties | null>(null);
  useEffect(() => {
    setFit(null);
    if (!src) return;
    let live = true;
    const img = new Image();
    img.onload = () => {
      if (!live) return;
      const canvas = document.createElement("canvas");
      canvas.width = img.naturalWidth;
      canvas.height = img.naturalHeight;
      const ctx = canvas.getContext("2d");
      if (!ctx || !canvas.width || !canvas.height) return;
      let box: Box | null = null;
      try {
        ctx.drawImage(img, 0, 0);
        box = alphaBounds(
          ctx.getImageData(0, 0, canvas.width, canvas.height).data,
          canvas.width,
          canvas.height,
        );
      } catch {
        return;
      }
      if (box) setFit(fitBox(box, canvas.width, canvas.height, FIT_W, FIT_H, PLATE, PLATE));
    };
    img.src = src;
    return () => {
      live = false;
    };
  }, [src]);
  return fit ?? UNFITTED;
}

export function GlyphPlate({ src, imgRef }: { src: string; imgRef?: RefObject<HTMLImageElement | null> }) {
  const fit = useGlyphFit(src);
  return (
    <span className="e-glyph-plate">
      {src && (
        <img
          ref={imgRef}
          src={src}
          alt=""
          draggable={false}
          style={{ position: "absolute", transformOrigin: "50% 50%", ...fit }}
        />
      )}
    </span>
  );
}

export function PackTile({
  selected,
  title,
  label,
  onPick,
  onHoverStart,
  onHoverEnd,
  children,
}: {
  selected: boolean;
  title: string;
  label?: string;
  onPick: () => void;
  onHoverStart?: () => void;
  onHoverEnd?: () => void;
  children: ReactNode;
}) {
  const still = useReducedMotion();
  return (
    <motion.button
      type="button"
      className={`e-tile${selected ? " on" : ""}`}
      title={title}
      aria-label={title}
      aria-pressed={selected}
      onClick={onPick}
      onHoverStart={onHoverStart}
      onHoverEnd={onHoverEnd}
      whileTap={still ? undefined : { scale: 0.96 }}
      transition={PRESS}
    >
      {children}
      {label && <span className="e-tile-label">{label}</span>}
    </motion.button>
  );
}
