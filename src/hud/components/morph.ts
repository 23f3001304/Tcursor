import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

const reduce = () =>
  typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
const easeOutCubic = (t: number) => 1 - Math.pow(1 - t, 3);

/** Smoothly resize the window from (fromW,fromH) to (toW,toH) over `ms`, top-left
 *  fixed, via requestAnimationFrame so the bar<->box change glides instead of
 *  jumping. Snaps instantly under prefers-reduced-motion. Resolves when done. */
export function morphWindow(fromW: number, fromH: number, toW: number, toH: number, ms: number): Promise<void> {
  const win = getCurrentWindow();
  if (reduce() || ms <= 0) { void win.setSize(new LogicalSize(toW, toH)); return Promise.resolve(); }
  return new Promise((resolve) => {
    let start = 0;
    const step = (now: number) => {
      if (!start) start = now;
      const k = easeOutCubic(Math.min(1, (now - start) / ms));
      void win.setSize(new LogicalSize(
        Math.round(fromW + (toW - fromW) * k),
        Math.round(fromH + (toH - fromH) * k),
      ));
      if ((now - start) / ms < 1) requestAnimationFrame(step);
      else resolve();
    };
    requestAnimationFrame(step);
  });
}
