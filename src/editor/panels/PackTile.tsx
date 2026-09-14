import { useEffect, useState, type CSSProperties, type ReactNode, type RefObject } from "react";
import { motion, useReducedMotion } from "motion/react";
import { alphaBounds, fitBox, type Box } from "./glyphFit";

// The panel's ONE card level, and since the arrangements pass every picture tile in the editor
// again: a cursor pack, a wallpaper and a gradient preset are all this tile (raised plane, one
// lightness step on hover, a 2px inset accent ring when chosen, the name captioned underneath).
// `.e-tile` lives in panels.css.
const PRESS = { type: "spring" as const, stiffness: 500, damping: 30 };
// The glyph plate, and the area a sprite's own drawing is fitted into inside it.
const PLATE = 34, FIT_W = 20, FIT_H = 24;
// Until (or unless) a sprite can be measured - jsdom, a blocked canvas read - draw it centred at
// a plain 22px, which is what the grid did before the fit existed.
const UNFITTED: CSSProperties = { width: 22, height: 22, left: (PLATE - 22) / 2, top: (PLATE - 22) / 2 };

/** Measure `src`'s drawn content once, on an offscreen canvas, and return the CSS that places the
 *  whole image so that content fills the plate's fit area. Re-runs only when `src` changes, so a
 *  grid of packs measures once per pack and a hover cycle (which swaps `src` on the element
 *  directly, never through React) never re-measures. */
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
        box = alphaBounds(ctx.getImageData(0, 0, canvas.width, canvas.height).data, canvas.width, canvas.height);
      } catch { return; } // a tainted canvas keeps the unfitted fallback rather than throwing
      if (box) setFit(fitBox(box, canvas.width, canvas.height, FIT_W, FIT_H, PLATE, PLATE));
    };
    img.src = src;
    return () => { live = false; };
  }, [src]);
  return fit ?? UNFITTED;
}

/** A cursor sprite on the shared neutral plate. The plate is what makes an ink-black pack legible
 *  on a dark tile - the glyph itself is never tinted, filtered or shadowed, so what the tile shows
 *  is the colour the pack actually draws. `imgRef` lets a caller drive the element directly (the
 *  pack grid's hover cycle writes `src`/`transform` on it from a rAF loop). */
export function GlyphPlate({ src, imgRef }: { src: string; imgRef?: RefObject<HTMLImageElement | null> }) {
  const fit = useGlyphFit(src);
  return (
    <span className="e-glyph-plate">
      {src && <img ref={imgRef} src={src} alt="" draggable={false}
        style={{ position: "absolute", transformOrigin: "50% 50%", ...fit }} />}
    </span>
  );
}

/** One choosable tile: a raised plane that lightens on hover and takes a 2px accent ring when
 *  chosen. Nothing here darkens or tints the tile's own content to mark selection. */
export function PackTile({ selected, title, label, onPick, onHoverStart, onHoverEnd, children }: {
  selected: boolean;
  /** Tooltip and accessible name - every tile is a picture, so it always needs one. */
  title: string;
  /** Caption UNDER the face. Every picker passes it: a name a user has to hover to read is the
   *  thing the arrangements pass removed, and a tooltip here was being clipped by its own row. */
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
