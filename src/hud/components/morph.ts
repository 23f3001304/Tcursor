import { getCurrentWindow, LogicalSize, PhysicalPosition } from "@tauri-apps/api/window";
import { setCapturable } from "../../lib/ipc";

const reduce = () =>
  typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
const easeOutCubic = (t: number) => 1 - Math.pow(1 - t, 3);

/** Which point of the window stays put while it changes size. `"corner"` is Tauri's own
 *  `setSize` behaviour (top-left fixed), the default; `"centre"` keeps the window's horizontal
 *  midpoint, so the bar shrinking into the take pill (and back) stays where the eye was instead of
 *  collapsing toward its left end - the one morph left, now that Settings opens inside the card. */
export type MorphAnchor = "corner" | "centre";

/** Re-assert that the HUD window is excluded from screen capture. Windows has been seen to drop
 *  the exclude-from-capture affinity across a resize (the take pill showed up in a display
 *  capture on 2026-09-14 while the idle card never did), so every morph and every snap resize
 *  ends here. Idempotent and one IPC; a failure is the backend's to log. Safe around the hand-off
 *  to the editor: the editor re-asserts the opposite on its own mount. */
export const keepHidden = (): Promise<void> => setCapturable(false).then(() => undefined, () => undefined);

/** Smoothly resize the window from (fromW,fromH) to (toW,toH) over `ms` via requestAnimationFrame
 *  so the change glides instead of jumping. With `anchor: "centre"` the window is also moved each
 *  frame so its horizontal centre never shifts (the position is read once, before the first
 *  frame, and the shift is half the width change in physical pixels). Snaps instantly under
 *  prefers-reduced-motion, position included. Resolves when done. */
export async function morphWindow(fromW: number, fromH: number, toW: number, toH: number, ms: number, anchor: MorphAnchor = "corner"): Promise<void> {
  const win = getCurrentWindow();
  const centre = anchor === "centre"
    ? await Promise.all([win.outerPosition(), win.scaleFactor()]).then(([p, k]) => ({ x: p.x, y: p.y, k }))
    : null;
  const place = (w: number) => centre ? win.setPosition(new PhysicalPosition(Math.round(centre.x + (fromW - w) * centre.k / 2), centre.y)) : Promise.resolve();
  if (reduce() || ms <= 0) { await win.setSize(new LogicalSize(toW, toH)); await place(toW); await keepHidden(); return; }
  await new Promise<void>((resolve) => {
    let start = 0;
    const step = (now: number) => {
      if (!start) start = now;
      const k = easeOutCubic(Math.min(1, (now - start) / ms));
      const w = Math.round(fromW + (toW - fromW) * k);
      void win.setSize(new LogicalSize(w, Math.round(fromH + (toH - fromH) * k)));
      void place(w);
      if ((now - start) / ms < 1) requestAnimationFrame(step);
      else resolve();
    };
    requestAnimationFrame(step);
  });
  await keepHidden();
}
