import type { EffectRegion } from "../lib/edit";

export interface SpotParams {
  dim: number;
  radius: number;
  feather: number;
  mode: string;
  tint: [number, number, number];
}

export interface SpotlightInput {
  effects: EffectRegion[];
  on: boolean;
  params: SpotParams;
}

export interface ResolvedSpotlight {
  alpha: number;
  mode: string;
  dim: number;
  radius: number;
  feather: number;
  tint: [number, number, number];
}

export interface SpotlightSimState {
  driver: string | null; // the winning EffectRegion's id, or null
  transitionFrom: number | null;
  transitionStart: number | null;
  transitionDur: number | null;
  alpha: number;
}

export function newSpotlightSimState(): SpotlightSimState {
  return { driver: null, transitionFrom: null, transitionStart: null, transitionDur: null, alpha: 0 };
}

function regionAlpha(e: EffectRegion, ms: number): number {
  if (ms < e.start_ms || ms >= e.end_ms) return 0;
  const inn = (ms - e.start_ms) / Math.max(1, e.fade_in_ms);
  const outn = (e.end_ms - ms) / Math.max(1, e.fade_out_ms);
  return Math.min(1, Math.max(0, Math.min(inn, outn)));
}

function winner(effects: EffectRegion[], ms: number): EffectRegion | null {
  let best: EffectRegion | null = null;
  for (const e of effects) {
    if (ms < e.start_ms || ms >= e.end_ms) continue;
    if (!best || (e.layer ?? 0) >= (best.layer ?? 0)) best = e; // later-in-array wins ties, matching Rust's max_by_key
  }
  return best;
}

/** Resolve spotlight alpha/look at `ms`, mutating `sim` in place (mirrors the Rust
 *  `SpotlightSim` in `fx_state.rs` exactly - both must change together). */
export function resolveSpotlight(spotlight: SpotlightInput, ms: number, sim: SpotlightSimState): ResolvedSpotlight | null {
  const { effects, on, params } = spotlight;
  const win = winner(effects, ms);
  if (win?.id !== sim.driver) {
    if (sim.driver !== null) {
      const outgoing = effects.find(e => e.id === sim.driver);
      const dur = win ? win.fade_in_ms : (outgoing?.fade_out_ms ?? 250);
      sim.transitionFrom = sim.alpha; sim.transitionStart = ms; sim.transitionDur = Math.max(1, dur);
    }
    sim.driver = win?.id ?? null;
  }
  const natural = win ? regionAlpha(win, ms) : 0;
  if (sim.transitionStart !== null && sim.transitionDur !== null && sim.transitionFrom !== null) {
    const elapsed = ms - sim.transitionStart;
    if (elapsed < sim.transitionDur) {
      const e = elapsed / sim.transitionDur; // Smooth easing, matches the Rust default
      const smooth = e * e * (3 - 2 * e);
      sim.alpha = sim.transitionFrom + (natural - sim.transitionFrom) * smooth;
    } else {
      sim.transitionStart = null; sim.transitionDur = null; sim.transitionFrom = null;
      sim.alpha = natural;
    }
  } else {
    sim.alpha = natural;
  }
  const alpha = Math.max(sim.alpha, on ? 1 : 0);
  if (alpha <= 0) return null;

  const mode = win?.mode && win.mode !== "global" ? win.mode : params.mode;
  const dim = win?.dim !== undefined && win.dim >= 0 ? win.dim : params.dim;
  const radius = win?.radius !== undefined && win.radius >= 0 ? win.radius : params.radius;
  const feather = win?.feather !== undefined && win.feather >= 0 ? win.feather : params.feather;
  return { alpha, mode, dim, radius, feather, tint: params.tint };
}
