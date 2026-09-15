import type { CamZoomAction, Zoom, ZoomTarget } from "../../shared/edit";
import type { GraphInput } from "../motion/graphModel";
import type { SegOption } from "./InspectorRows";

export function zoomGraphInput(zoom: Zoom, zooms: Zoom[]): GraphInput {
  const before = zooms
    .filter((z) => z.end_ms <= zoom.start_ms && z.id !== zoom.id)
    .sort((a, b) => b.end_ms - a.end_ms)[0];
  const after = zooms
    .filter((z) => z.start_ms >= zoom.end_ms && z.id !== zoom.id)
    .sort((a, b) => a.start_ms - b.start_ms)[0];
  return {
    lane: "zoom",
    startMs: zoom.start_ms,
    endMs: zoom.end_ms,
    peak: zoom.scale,
    rampIn: { easing: zoom.easing, durMs: zoom.zoom_in_ms },
    rampOut: { easing: zoom.easing_out ?? zoom.easing, durMs: zoom.zoom_out_ms },
    followHint: targetMode(zoom.target) === "cursor",
    prev: before
      ? { endMs: before.end_ms, easingOut: before.easing_out ?? before.easing, durMs: before.zoom_out_ms }
      : null,
    next: after ? { startMs: after.start_ms, easingIn: after.easing, durMs: after.zoom_in_ms } : null,
  };
}

export type TargetMode = "cursor" | "region";
export const targetMode = (t: ZoomTarget): TargetMode => (t === "cursor" ? "cursor" : "region");

export function targetForMode(mode: TargetMode, current: ZoomTarget): ZoomTarget {
  if (mode === "cursor") return "cursor";
  return typeof current === "object" ? current : { fixed: { x: 0.5, y: 0.5 } };
}

export const CAM_ACTION_OPTIONS: { label: string; value: CamZoomAction | null }[] = [
  { label: "Global default", value: null },
  { label: "Stay", value: "stay" },
  { label: "Shrink", value: { shrink: { to: 0.62 } } },
  { label: "Hide", value: "hide" },
];

export function isCamActionSelected(
  current: CamZoomAction | null | undefined,
  option: CamZoomAction | null,
): boolean {
  if (option === null) return current == null;
  if (typeof option === "string") return current === option;
  return typeof current === "object" && current !== null && "shrink" in current;
}

export function zoomScopedSeekMs(nowMs: number, startMs: number, endMs: number): number | null {
  if (nowMs >= startMs && nowMs <= endMs) return null;
  return Math.round((startMs + endMs) / 2);
}

export function durationOptions(smart: boolean): SegOption[] {
  return [
    { key: "fixed", label: "Fixed", on: !smart, title: "The end stays where you put it" },
    { key: "smart", label: "Smart typing", on: smart, title: "The end follows the typing after the start" },
  ];
}
