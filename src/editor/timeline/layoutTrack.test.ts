import { describe, it, expect } from "vitest";
import { fitDurations, layoutAt } from "./layoutTrack";
import type { LayoutSeg } from "../../lib/edit";
import type { LayoutPresetDto, LayoutPresets, SegRectDto } from "../../lib/ipc";

const CANVAS: [number, number] = [1280, 720];

const panel = (x: number, y: number, w: number, h: number): LayoutPresetDto["screen"] =>
  ({ rect: [x, y, w, h], radius: 0.01, alpha: 1, ring_px: 0, ring_color: [0, 0, 0] });
// `arrangement` rides on the same DTO (the preset expressed as poses); layoutAt never reads it,
// so the fixture carries the shape the wire really has without exercising it.
const preset = (sx: number, cx: number): LayoutPresetDto => ({
  screen: panel(sx, 0, 0.8, 0.8), cam: panel(cx, 0.7, 0.2, 0.2),
  arrangement: { screen: { cx: sx + 0.4, cy: 0.4, size: 0.8 }, cam: { cx: cx + 0.1, cy: 0.8, size: 0.2 } },
});
// Deliberately distinct x offsets so any blend is visible in `screen[0]`.
const PRESETS: LayoutPresets = {
  screen: preset(0.1, 0.1), camera: preset(0.5, 0.5), presenter: preset(0.9, 0.9),
  screen_only: preset(0.3, 0.3), camera_only: preset(0.7, 0.7),
  segs: [], // no per-segment overrides by default - every fixture segment resolves from its preset
  // A take that never switched display: one full-canvas span, no panel squeeze - so every
  // assertion below is about the layout track alone, exactly as before spans existed.
  spans: [{ start_ms: 0, src: [0, 0, 1, 1], transition_ms: 350, fit: [1, 1] }],
  inset_w: 0.8, // unused by layoutAt (cursorPanel.ts's concern) - matches the fixture screens' own width
};

const seg = (id: string, start_ms: number, end_ms: number, layout: string,
  transition_ms = 0, transition_out_ms = 0): LayoutSeg =>
  ({ id, start_ms, end_ms, layout, transition_ms, easing: "smooth", transition_out_ms, easing_out: "smooth" });

// A per-segment override at an x offset unlike ANY preset's (0.1/0.3/0.5/0.7/0.9), so a resolved
// screen[0] equal to it is unmistakably the SEGMENT's own rect, not its provenance preset's.
const segRect = (id: string, sx: number, cx: number): SegRectDto =>
  ({ id, screen: panel(sx, 0, 0.8, 0.8), cam: panel(cx, 0.7, 0.2, 0.2) });

const sx = (segs: LayoutSeg[], t: number) => layoutAt(segs, PRESETS, t, CANVAS)!.screen[0];
const sxWith = (segs: LayoutSeg[], presets: LayoutPresets, t: number) => layoutAt(segs, presets, t, CANVAS)!.screen[0];

describe("layoutAt exit transitions (T33 - parity with LayoutTrack::scene_at)", () => {
  it("returns null until the presets load", () => {
    expect(layoutAt([seg("l0", 0, 1000, "camera")], null, 0, CANVAS)).toBeNull();
  });

  it("treats a 0 transition_out_ms as a hard cut, leaving old docs unchanged", () => {
    const segs = [seg("l0", 0, 1000, "camera"), seg("l1", 2000, 3000, "presenter")];
    expect(sx(segs, 999)).toBeCloseTo(0.5, 6);  // last active ms is still fully camera
    expect(sx(segs, 1000)).toBeCloseTo(0.1, 6); // and it cuts to the gap default (screen)
  });

  it("completes the exit blend exactly at end_ms", () => {
    // camera [0,1000) with a 400ms exit, then a GAP -> it eases back to the screen default.
    const segs = [seg("l0", 0, 1000, "camera", 0, 400)];
    expect(sx(segs, 599)).toBeCloseTo(0.5, 6); // before the window
    expect(sx(segs, 600)).toBeCloseTo(0.5, 6); // window opens at f=0
    const mid = sx(segs, 800);
    expect(mid).toBeGreaterThan(0.1);
    expect(mid).toBeLessThan(0.5);             // strictly between camera and screen
    expect(Math.abs(sx(segs, 999) - sx(segs, 1000))).toBeLessThan(1e-4); // lands ON the successor
  });

  it("lets a gapless successor's entry win the overlap - no double blend, no backwards jump", () => {
    const segs = [seg("l0", 0, 1000, "camera", 0, 400), seg("l1", 1000, 2000, "presenter", 400)];
    for (const t of [600, 800, 999]) expect(sx(segs, t)).toBeCloseTo(0.5, 6); // exit suppressed
    expect(sx(segs, 1000)).toBeCloseTo(0.5, 6);  // the entry starts FROM camera - seamless
    expect(sx(segs, 1400)).toBeCloseTo(0.9, 6);  // and completes into presenter
  });

  it("still runs the exit into a successor that hard-cuts in", () => {
    const segs = [seg("l0", 0, 1000, "camera", 0, 400), seg("l1", 1000, 2000, "presenter", 0)];
    expect(sx(segs, 800)).not.toBeCloseTo(0.5, 3);
    expect(Math.abs(sx(segs, 999) - 0.9)).toBeLessThan(1e-3); // converged on presenter
  });

  // Was "resolves ... by entry-first": the entry used to simply outrank the exit for the whole
  // span, so a segment asking for more transition than it has ramped in across ALL of it, never
  // arrived, and then hard-cut back at `end_ms`. Transitions are now fitted into the span first
  // (`fitDurations`, mirroring Rust `from_segs`), which makes the same segment a symmetric bump.
  it("gives a segment shorter than its own two transitions a symmetric in-and-out", () => {
    const segs = [seg("l0", 1000, 1300, "camera", 300, 300)]; // 600ms asked for inside 300ms
    expect(sx(segs, 1000)).toBeCloseTo(0.1, 6); // entry at f=0, from the screen base
    expect(sx(segs, 1150)).toBeCloseTo(0.5, 6); // 150ms entry ARRIVES at camera, at the midpoint
    expect(sx(segs, 1299)).toBeCloseTo(0.1, 2); // 150ms exit is back on the base by end_ms
    // and it is genuinely a round trip, not a ramp that overshoots and snaps
    expect(sx(segs, 1075)).toBeLessThan(0.5);
    expect(sx(segs, 1225)).toBeLessThan(0.5);
  });
});

describe("layoutAt per-segment resolved rects (T34 L2 - posed segments preview their own panels)", () => {
  it("falls back to the preset lookup when a segment has no segs entry", () => {
    const segs = [seg("l0", 0, 1000, "camera")];
    // An entry exists in `segs`, but for a DIFFERENT id - proves the lookup is keyed on id, not
    // "any entry present at all" or array order.
    const withSegs: LayoutPresets = { ...PRESETS, segs: [segRect("other-id", 0.99, 0.99)] };
    expect(sxWith(segs, withSegs, 500)).toBeCloseTo(0.5, 6); // untouched: still the "camera" preset
  });

  it("shows a posed segment's own resolved rect, not its provenance preset's", () => {
    const segs = [seg("l0", 0, 1000, "camera")]; // provenance preset "camera" would resolve to 0.5
    const withSegs: LayoutPresets = { ...PRESETS, segs: [segRect("l0", 0.42, 0.42)] };
    expect(sxWith(segs, withSegs, 500)).toBeCloseTo(0.42, 6);
  });

  it("crossfades INTO a posed segment exactly like preset-to-preset, landing on the pose", () => {
    // Starts at 1000 (not 0) so the "just before this seg" probe lands in the GAP, not inside the
    // segment's own [start, end) - the same reason the pre-existing exit-transition tests above
    // avoid a segment starting exactly at t=0.
    const segs = [seg("l0", 1000, 2000, "camera", 400)]; // entry from the gap default (screen, 0.1)
    const withSegs: LayoutPresets = { ...PRESETS, segs: [segRect("l0", 0.42, 0.42)] };
    expect(sxWith(segs, withSegs, 1000)).toBeCloseTo(0.1, 6); // f=0: still the outgoing default
    const mid = sxWith(segs, withSegs, 1200);
    expect(mid).toBeGreaterThan(0.1);
    expect(mid).toBeLessThan(0.42); // strictly between the gap default and the pose
    expect(sxWith(segs, withSegs, 1400)).toBeCloseTo(0.42, 5); // lands on the POSE, not "camera"'s 0.5
  });

  it("crossfades OUT of a posed segment into a plain preset segment's own rect", () => {
    const segs = [seg("l0", 0, 1000, "camera", 0, 400), seg("l1", 1000, 2000, "presenter")];
    const withSegs: LayoutPresets = { ...PRESETS, segs: [segRect("l0", 0.42, 0.42)] };
    expect(sxWith(segs, withSegs, 599)).toBeCloseTo(0.42, 6); // before the exit window: still the pose
    const mid = sxWith(segs, withSegs, 800);
    expect(mid).toBeGreaterThan(0.42);
    expect(mid).toBeLessThan(0.9); // strictly between the pose and "presenter"'s preset rect
    expect(Math.abs(sxWith(segs, withSegs, 999) - sxWith(segs, withSegs, 1000))).toBeLessThan(1e-4); // lands smoothly on the successor
  });

  it("blends two DIFFERENT posed segments by their own rects, not by preset name", () => {
    // Both segments share the "camera" provenance preset (0.5) but are posed to distinct rects -
    // a naive preset-name blend would show 0.5 throughout; the real one must move between the poses.
    const segs = [seg("l0", 0, 1000, "camera", 0, 400), seg("l1", 1000, 2000, "camera", 400)];
    const withSegs: LayoutPresets = { ...PRESETS, segs: [segRect("l0", 0.2, 0.2), segRect("l1", 0.8, 0.8)] };
    expect(sxWith(segs, withSegs, 999)).toBeCloseTo(0.2, 5);
    expect(sxWith(segs, withSegs, 1400)).toBeCloseTo(0.8, 5);
    const mid = sxWith(segs, withSegs, 1200);
    expect(mid).toBeGreaterThan(0.2);
    expect(mid).toBeLessThan(0.8);
  });
});

describe("layoutAt fits transitions into their segment (parity with LayoutTrack::from_segs)", () => {
  it("shrinks an entry+exit pair that would outlast the span, and leaves a fitting pair alone", () => {
    expect(fitDurations(350, 0, 1000)).toEqual([350, 0]);   // fits: untouched
    expect(fitDurations(350, 350, 700)).toEqual([350, 350]); // exactly fits: untouched
    expect(fitDurations(350, 350, 400)).toEqual([200, 200]); // halved to fit
    expect(fitDurations(0, 0, 0)).toEqual([0, 0]);           // no divide-by-zero on an empty pair
  });

  it("hands a short segment over to the next one without a jump", () => {
    // A 200ms segment carrying a 350ms entry. Unfitted it only ever got ~60% of the way to its own
    // scene, yet the next segment's entry blended FROM that full unreached scene - so screen[0]
    // snapped 0.3409 -> 0.5000 in a single frame at the boundary.
    const segs = [seg("a", 0, 1000, "screen"), seg("b", 1000, 1200, "camera", 350), seg("c", 1200, 3000, "presenter", 350)];
    const step = Math.abs(sx(segs, 1199) - sx(segs, 1180));
    const boundary = Math.abs(sx(segs, 1200) - sx(segs, 1199));
    expect(boundary).toBeLessThanOrEqual(Math.max(step, 0.001) * 2);
    // It arrives: the fitted entry lands exactly AT end_ms, so its last frame is on its own scene.
    expect(sx(segs, 1199)).toBeCloseTo(PRESETS.camera.screen.rect[0], 3);
  });

  it("never lets a fitted entry and exit overlap inside a short segment", () => {
    // 700ms of transition asked for inside a 400ms segment: unfitted, the exit ran from t=50 under
    // an entry that owned the frame to t=350, so the scene lurched to the successor in one frame.
    const segs = [seg("a", 0, 1000, "screen"), seg("b", 1000, 1400, "camera", 350, 350), seg("c", 1400, 3000, "presenter")];
    let worst = 0;
    for (let t = 1000; t < 1400; t++) worst = Math.max(worst, Math.abs(sx(segs, t + 1) - sx(segs, t)));
    expect(worst).toBeLessThan(0.01);
  });
});
