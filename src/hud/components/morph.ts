import { getCurrentWindow, LogicalSize, PhysicalPosition } from "@tauri-apps/api/window";
import { setCapturable } from "../../shared/ipc";

const reduce = () =>
  typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
const easeOutCubic = (t: number) => 1 - Math.pow(1 - t, 3);

export type MorphAnchor = "corner" | "centre";

export const keepHidden = (): Promise<void> =>
  setCapturable(false).then(
    () => undefined,
    () => undefined,
  );

export async function morphWindow(
  fromW: number,
  fromH: number,
  toW: number,
  toH: number,
  ms: number,
  anchor: MorphAnchor = "corner",
): Promise<void> {
  const win = getCurrentWindow();
  const centre =
    anchor === "centre"
      ? await Promise.all([win.outerPosition(), win.scaleFactor()]).then(([p, k]) => ({ x: p.x, y: p.y, k }))
      : null;
  const place = (w: number) =>
    centre
      ? win.setPosition(new PhysicalPosition(Math.round(centre.x + ((fromW - w) * centre.k) / 2), centre.y))
      : Promise.resolve();
  if (reduce() || ms <= 0) {
    await win.setSize(new LogicalSize(toW, toH));
    await place(toW);
    await keepHidden();
    return;
  }
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
