import type { CSSProperties } from "react";

const REF_W = 1440,
  REF_H = 900;

export const MIN_SCALE = 0.86;

export const NARROW_W = 1420,
  TIGHT_W = 1400,
  CRAMPED_W = 1100;

export const SHORT_H = 820,
  CRAMPED_H = 640;

export interface Density {
  scale: number;
  rail: number;
  railBtn: number;
  panel: number;
  side: number;
  gutter: number;
  film: number;
  row: number;
  audioRow: number;
  gap: number;
  tracks: number;
  narrow: boolean;
  tight: boolean;
  cramped: boolean;
  short: boolean;
}

export function densityFor(width: number, height: number): Density {
  const raw = Math.min(width / REF_W, height / REF_H);
  const clamped = Math.min(1, Math.max(MIN_SCALE, Number.isFinite(raw) ? raw : 1));
  const cramped = width < CRAMPED_W || height < CRAMPED_H;
  const tight = cramped || width < TIGHT_W;
  const narrow = tight || width < NARROW_W;
  const short = cramped || height < SHORT_H;
  return {
    scale: Math.round(clamped * 100) / 100,
    rail: cramped ? 44 : tight ? 48 : 56,
    railBtn: cramped ? 34 : tight ? 36 : 42,
    panel: cramped ? 248 : tight ? 288 : 320,
    side: cramped ? 240 : tight ? 288 : narrow ? 320 : 360,
    gutter: tight ? 60 : 72,
    film: cramped ? 52 : short ? 64 : 80,
    row: short ? 28 : 32,
    audioRow: short ? 20 : 22,
    gap: short ? 5 : 6,
    tracks: cramped ? 120 : short ? 168 : 230,
    narrow,
    tight,
    cramped,
    short,
  };
}

export function densityVars(d: Density): CSSProperties {
  return {
    "--e-density": String(d.scale),
    "--e-rail-w": `${d.rail}px`,
    "--e-rail-btn": `${d.railBtn}px`,
    "--e-panel-w": `${d.panel}px`,
    "--e-side-w": `${d.side}px`,
    "--e-gutter-w": `${d.gutter}px`,
    "--e-film-h": `${d.film}px`,
    "--e-row-h": `${d.row}px`,
    "--e-audio-h": `${d.audioRow}px`,
    "--e-row-gap": `${d.gap}px`,
    "--e-tracks-h": `${d.tracks}px`,
  } as CSSProperties;
}

export function sameDensity(a: Density, b: Density): boolean {
  return (
    a.scale === b.scale &&
    a.rail === b.rail &&
    a.railBtn === b.railBtn &&
    a.panel === b.panel &&
    a.side === b.side &&
    a.gutter === b.gutter &&
    a.film === b.film &&
    a.row === b.row &&
    a.audioRow === b.audioRow &&
    a.gap === b.gap &&
    a.tracks === b.tracks
  );
}
