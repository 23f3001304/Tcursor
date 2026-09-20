import type { Clip } from "../../../shared/edit";
import { framePlan, type TimeMap } from "../../../shared/math/remap";
import { clipSpans } from "../../../shared/math/remapPlan";
import { ease } from "../../timeline/model/layoutTrack";

export const PREVIEW_FPS = 60;

export const PRESEEK_LEAD_MS = 1000;

export interface ClipDissolve {
  outStartMs: number;
  prevClip: number;
  durMs: number;
}

export interface ClipMix {
  prevClip: number;
  prevOutMs: number;
  alpha: number;
}

export function clipDissolves(map: TimeMap, clips: Clip[], fps = PREVIEW_FPS): ClipDissolve[] {
  const spans = clipSpans(map, fps);
  const out: ClipDissolve[] = [];
  for (let i = 1; i < spans.length; i++) {
    const want = clips[spans[i].clip]?.transition_in_ms ?? 0;
    const durMs = Math.min(want, Math.floor((spans[i].planLen * 1000) / fps));
    if (durMs <= 0) continue;
    out.push({
      outStartMs: Math.floor((spans[i].planStart * 1000) / fps),
      prevClip: spans[i - 1].clip,
      durMs,
    });
  }
  return out;
}

export function clipMixAt(list: ClipDissolve[], outMs: number, easing: string): ClipMix | null {
  const d = list.find((x) => outMs >= x.outStartMs && outMs < x.outStartMs + x.durMs);
  if (!d) return null;
  return {
    prevClip: d.prevClip,
    prevOutMs: Math.max(0, d.outStartMs - 1),
    alpha: ease(easing, (outMs - d.outStartMs) / d.durMs),
  };
}

export function preseekAt(
  list: ClipDissolve[],
  map: TimeMap,
  outMs: number,
  fps = PREVIEW_FPS,
  leadMs = PRESEEK_LEAD_MS,
): number | null {
  const d = list.find((x) => outMs >= x.outStartMs - leadMs && outMs < x.outStartMs + x.durMs);
  if (!d) return null;
  const jPrev = Math.floor((Math.max(0, d.outStartMs - 1) * fps) / 1000);
  const k = framePlan(map, fps)[jPrev];
  return k === undefined ? null : (k * 1000 + 500) / fps;
}
