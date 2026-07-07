/** Shared easing-curve set for the camera-move transition picker: the keyframe inspector renders
 *  these as full cards, the timeline segment popover as compact glyph buttons. `key` is the wire
 *  name the ops (`update_camera_move.easing`) and the shared `ease` (layoutTrack.ts / the Rust
 *  export::camera::ease) both understand - keep this list in lockstep with those. Add a curve here
 *  once and it shows in both places; but a new key ALSO needs an `ease` branch + valid_easing arm. */
export interface CurveDef { key: string; name: string; path: string }

export const CAM_CURVES: CurveDef[] = [
  { key: "linear", name: "Linear", path: "M 0 100 L 100 0" },
  { key: "ease_in", name: "Ease In", path: "M 0 100 C 55 98, 82 55, 100 0" },
  { key: "ease_out", name: "Ease Out", path: "M 0 100 C 18 45, 45 2, 100 0" },
  { key: "ease_in_out", name: "In / Out", path: "M 0 100 C 45 100, 55 0, 100 0" },
  { key: "smooth", name: "Smooth", path: "M 0 100 C 35 100, 65 0, 100 0" },
  { key: "spring", name: "Spring", path: "M 0 100 C 25 100, 45 -25, 75 -25 C 85 -25, 90 0, 100 0" },
];
