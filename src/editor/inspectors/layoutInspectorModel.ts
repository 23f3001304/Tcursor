import type { LayoutSeg } from "../../shared/edit";
import type { LayoutPresetName } from "../../shared/ipc";
import type { GraphInput } from "../motion/graphModel";

export function layoutGraphInput(seg: LayoutSeg, segs: LayoutSeg[]): GraphInput {
  const before = segs
    .filter((s) => s.end_ms <= seg.start_ms && s.id !== seg.id)
    .sort((a, b) => b.end_ms - a.end_ms)[0];
  const after = segs
    .filter((s) => s.start_ms >= seg.end_ms && s.id !== seg.id)
    .sort((a, b) => a.start_ms - b.start_ms)[0];
  return {
    lane: "layout",
    startMs: seg.start_ms,
    endMs: seg.end_ms,
    peak: 1,
    rampIn: { easing: seg.easing, durMs: seg.transition_ms },
    rampOut: { easing: seg.easing_out, durMs: seg.transition_out_ms },
    prev: before
      ? { endMs: before.end_ms, easingOut: before.easing_out, durMs: before.transition_out_ms }
      : null,
    next: after ? { startMs: after.start_ms, easingIn: after.easing, durMs: after.transition_ms } : null,
  };
}

export const LAYOUT_PRESETS: { value: LayoutPresetName; label: string }[] = [
  { value: "camera", label: "Camera" },
  { value: "presenter", label: "Presenter" },
  { value: "screen_only", label: "Screen only" },
  { value: "camera_only", label: "Camera only" },
];

export const prettyLayout = (v: string) => v.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());
