export const RIPPLE_FROM = 8;
export const RIPPLE_TO = 56;
export const RIPPLE_MS = 320;

export const RIPPLE_CAP = 6;

export type RippleTone = "rim" | "accent";

export interface Ripple {
  id: number;
  x: number;
  y: number;
  tone: RippleTone;
}

export const FX_OFF_ATTR = 'data-ui-fx="off"';
const FX_OFF_SELECTOR = `[${FX_OFF_ATTR}]`;

const ACCENT_SELECTOR = ".e-play, .e-export, .on, [aria-current]";

export function suppressesRipple(el: Element | null): boolean {
  return el !== null && el.closest(FX_OFF_SELECTOR) !== null;
}

export function rippleTone(el: Element | null): RippleTone {
  return el !== null && el.closest(ACCENT_SELECTOR) !== null ? "accent" : "rim";
}

export function pushRipple(list: readonly Ripple[], r: Ripple): Ripple[] {
  const next = [...list, r];
  return next.length > RIPPLE_CAP ? next.slice(next.length - RIPPLE_CAP) : next;
}

export function dropRipple(list: Ripple[], id: number): Ripple[] {
  return list.some((r) => r.id === id) ? list.filter((r) => r.id !== id) : list;
}
