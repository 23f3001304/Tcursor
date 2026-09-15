export type Lane = "zoom" | "fx" | "trim-in" | "trim-out";

const ROW_SELECTOR: Partial<Record<Lane, string>> = { zoom: ".e-zoomrow", fx: ".e-fxrow" };

export function timelinePointForMs(
  trackEl: HTMLElement,
  ms: number,
  dur: number,
  lane: Lane,
): { x: number; y: number } {
  const rect = trackEl.getBoundingClientRect();
  const frac = dur > 0 ? Math.min(1, Math.max(0, ms / dur)) : 0;
  const x = rect.left + frac * rect.width;

  const sel = ROW_SELECTOR[lane];
  const rows = sel ? trackEl.querySelectorAll<HTMLElement>(sel) : null;
  const row = rows && rows.length ? rows[rows.length - 1] : null;
  const y = row ? rectCenter(row).y : rect.top + rect.height / 2;
  return { x, y };
}

export function rectCenter(el: Element): { x: number; y: number } {
  const r = el.getBoundingClientRect();
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
}

export function anchorPoint(name: string): { x: number; y: number } | null {
  const el = document.querySelector(`[data-director-anchor="${name}"]`);
  return el ? rectCenter(el) : null;
}
