import { useRef } from "react";
import { getCurrentWindow, LogicalSize, LogicalPosition } from "@tauri-apps/api/window";

type Dir = "n" | "s" | "e" | "w" | "nw" | "ne" | "sw" | "se";
const GRIPS: Dir[] = ["n", "s", "e", "w", "nw", "ne", "sw", "se"];
const MIN_W = 880, MIN_H = 560;

/** Invisible resize grips on the editor's 4 edges + 4 corners. The window is frameless
 *  (decorations:false) AND transparent, so the OS gives it no resize border and native
 *  startResizeDragging is ignored. Instead each grip drives the resize itself with
 *  setSize/setPosition - the only window ops proven to work on this window - rAF-batched
 *  (the same loop the HUD morph uses) so it stays smooth. */
export function ResizeEdges() {
  const win = getCurrentWindow();
  // Logical geometry + pointer origin captured at drag start.
  const drag = useRef<{ dir: Dir; px: number; py: number; x: number; y: number; w: number; h: number } | null>(null);
  const pending = useRef<{ w: number; h: number; x: number; y: number; move: boolean } | null>(null);
  const raf = useRef(0);

  const flush = () => {
    raf.current = 0;
    const n = pending.current; if (!n) return;
    pending.current = null;
    void win.setSize(new LogicalSize(n.w, n.h));
    if (n.move) void win.setPosition(new LogicalPosition(n.x, n.y));
  };

  const onDown = async (e: React.PointerEvent, dir: Dir) => {
    if (e.button !== 0) return;
    e.preventDefault();
    (e.target as Element).setPointerCapture(e.pointerId);
    const sf = await win.scaleFactor();
    const p = await win.outerPosition(), s = await win.outerSize();
    drag.current = { dir, px: e.screenX, py: e.screenY, x: p.x / sf, y: p.y / sf, w: s.width / sf, h: s.height / sf };
  };

  const onMove = (e: React.PointerEvent) => {
    const d = drag.current; if (!d) return;
    const dx = e.screenX - d.px, dy = e.screenY - d.py;
    let w = d.w, h = d.h, x = d.x, y = d.y;
    if (d.dir.includes("e")) w = Math.max(MIN_W, d.w + dx);
    if (d.dir.includes("w")) { w = Math.max(MIN_W, d.w - dx); x = d.x + (d.w - w); }
    if (d.dir.includes("s")) h = Math.max(MIN_H, d.h + dy);
    if (d.dir.includes("n")) { h = Math.max(MIN_H, d.h - dy); y = d.y + (d.h - h); }
    pending.current = { w, h, x, y, move: d.dir.includes("w") || d.dir.includes("n") };
    if (!raf.current) raf.current = requestAnimationFrame(flush);
  };

  const onUp = (e: React.PointerEvent) => {
    drag.current = null;
    try { (e.target as Element).releasePointerCapture(e.pointerId); } catch { /* already released */ }
  };

  return (
    <>
      {GRIPS.map((dir) => (
        <div key={dir} className={`e-rz e-rz-${dir}`}
          onPointerDown={(e) => void onDown(e, dir)} onPointerMove={onMove}
          onPointerUp={onUp} onPointerCancel={onUp} />
      ))}
    </>
  );
}
