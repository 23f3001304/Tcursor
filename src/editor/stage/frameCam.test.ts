import { describe, it, expect } from "vitest";
import { activeCamDraft, frameCamLayout } from "./frameCam";
import type { PreviewLayout } from "../../lib/ipc";
import type { Zoom } from "../../lib/edit";
import type { CamPose } from "./cameraMoves";
import type { ZoomSettings } from "../../hud/settings/settings";

const OW = 1280, OH = 720;

// cam[2] (0.18) < screen[2] (0.7) - narrower than the screen panel, so the smart zoom action
// guard in frameCamLayout is satisfied and the "otherwise" branch (resolveCamAction +
// applyCamZoomAction/camZoomAlpha) actually runs.
const layout = (over: Partial<PreviewLayout> = {}): PreviewLayout => ({
  screen: [0.05, 0.05, 0.7, 0.9], radius: 0.02,
  cam: [0.78, 0.7, 0.18, 0.24, 0.02, 0, 255, 255, 255],
  canvas: [OW, OH],
  ...over,
});

const settings = (over: Partial<ZoomSettings> = {}): ZoomSettings => ({
  enabled: true, target_scale: 2.2, hold_ms: 2200, smoothness: 0.1, clicks: 1,
  camera_shrink: false, camera_shrink_min: 0.62, smart_hold: true, smart_follow: false,
  cam_zoom_default: "hide", ...over,
});

const zoom = (over: Partial<Zoom> = {}): Zoom => ({
  id: "z0", start_ms: 0, end_ms: 5000, target: "cursor", scale: 2.2, easing: "smooth",
  zoom_in_ms: 350, zoom_out_ms: 450, layer: 0, ...over,
});

// H2: `applyCamZoomAction` only ever touched geometry - `camZoomAlpha` had zero production
// callers, so "Webcam during zoom -> Hide" faded the PiP in the export but left it fully opaque
// in the preview. These pin `frameCamLayout` actually applying the alpha half too.
describe("frameCamLayout - camAlpha parity with the export (H2)", () => {
  it("fades camAlpha to 0 at full zoom when the resolved action is hide", () => {
    const out = frameCamLayout(layout(), 1000, 2.2, [], null, [zoom()], settings(), OW, OH);
    expect(out?.camAlpha).toBeCloseTo(0, 10);
  });

  it("leaves camAlpha at 1 for stay/shrink actions - only hide fades", () => {
    const out = frameCamLayout(layout(), 1000, 2.2, [], null, [zoom({ cam_action: "stay" })], settings(), OW, OH);
    expect(out?.camAlpha ?? 1).toBe(1);
  });

  it("multiplies onto an existing layout-transition alpha rather than overriding it", () => {
    const out = frameCamLayout(layout({ camAlpha: 0.5 }), 1000, 2.2, [], null, [zoom()], settings(), OW, OH);
    expect(out?.camAlpha).toBeCloseTo(0, 10);
  });

  it("leaves geometry alone - hide is a pure alpha effect", () => {
    const base = layout();
    const out = frameCamLayout(base, 1000, 2.2, [], null, [zoom()], settings(), OW, OH);
    expect(out?.cam).toEqual(base.cam);
  });

  it("skips the alpha adjustment while a camera_moves keyframe/drag owns the frame, matching Rust's `!keyframed` gate", () => {
    const drag = { x: 0.5, y: 0.5, size: 0.2 };
    const out = frameCamLayout(layout(), 1000, 2.2, [], drag, [zoom()], settings(), OW, OH);
    expect(out?.camAlpha ?? 1).toBe(1); // untouched even though the zoom would otherwise fully hide it
  });

  it("is a no-op when there is no camera panel", () => {
    expect(frameCamLayout(null, 1000, 2.2, [], null, [], settings(), OW, OH)).toBeNull();
  });
});

// T34 L3 review round 2: arrange mode must not let a leftover Move-mode draft pin the composited
// webcam (the draft outranks the base layout rect, which is where the arrange draft lives) - but
// it must not DESTROY that draft either. `CameraPanel` promises exactly two discard triggers:
// pressing Add/Update, or moving the playhead. Selecting a layout segment is not one of them.
describe("activeCamDraft - arrange mode suppresses the Move draft without discarding it", () => {
  const draft: CamPose = { x: 0.3, y: 0.4, size: 0.2 };

  it("hands the draft straight through while arrange mode is off", () => {
    expect(activeCamDraft(draft, false)).toBe(draft);
    expect(activeCamDraft(null, false)).toBeNull();
  });

  it("hides it from the composite while arrange mode is on", () => {
    expect(activeCamDraft(draft, true)).toBeNull();
  });

  it("entering AND leaving leaves a pending draft intact - the ref is never written", () => {
    const camDraftRef: { current: CamPose | null } = { current: { ...draft } };
    expect(activeCamDraft(camDraftRef.current, true)).toBeNull(); // arrange entry: ignored...
    expect(camDraftRef.current).toEqual(draft);                   // ...but still there
    expect(activeCamDraft(camDraftRef.current, false)).toEqual(draft); // exit: it reasserts
  });

  it("through the real composite path: the PiP holds its layout rect while arranging, and follows the draft after", () => {
    const base = layout();
    const drawn = (arranging: boolean) =>
      frameCamLayout(base, 1000, 1, [], activeCamDraft(draft, arranging), [], settings(), OW, OH)?.cam;
    expect(drawn(true)).toEqual(base.cam); // suppressed - the arrange draft in `base` wins
    expect(drawn(false)).not.toEqual(base.cam); // reasserted once arranging ends
  });
});
