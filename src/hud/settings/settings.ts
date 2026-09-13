
export type CamShape = "circle" | "rounded" | "rect";
export type CamCorner = "bottom_left" | "bottom_right" | "top_left" | "top_right";
export type CamAspect = "square" | "wide";
export interface CamRing { width: number; color: [number, number, number] }
export interface ModeAppearance {
  pad: number; screen_size: number; screen_radius: number; cam_size: number;
  cam_shape: CamShape; cam_radius: number; cam_corner: CamCorner;
  cam_margin_x: number; cam_margin_y: number;
  cam_aspect: CamAspect; cam_ring: CamRing | null;
}
export interface AppearanceSettings {
  screen: ModeAppearance; screen_only: ModeAppearance; camera: ModeAppearance;
  camera_only: ModeAppearance; presenter: ModeAppearance;
}
export type ThemeMode = "light" | "dark" | "system";
/** `animated_brand` (Task 39) - the living-brand feel knob: whether `TcursorMark` flows/pulses
 *  for its recording/exporting/directing states, in the HUD and the editor's TopBar. `false`
 *  and `prefers-reduced-motion` both fall the mark back to its static idle rendering. */
export interface InterfaceSettings { theme: ThemeMode; accent: [number, number, number]; animated_brand: boolean }
export type CursorStyle = "system" | "enhanced" | "hidden";
export interface CursorSettings { style: CursorStyle; size: number; smoothness: number; path_idealize: number; motion_blur: number; click_bounce: boolean; bounce_intensity: number; pack: string }
export type ClickFxStyle = "none" | "ripple" | "pulse" | "glow" | "shockwave" | "particles" | "neon";
export type SpotlightMode = "classic" | "blur" | "halo" | "breathing" | "nebula" | "vignette";
export type VideoFxMode = "nebulawash" | "cinematicdim" | "screenfocus" | "colorpop";
/** What the webcam PiP does while a zoom is active. Wire form of Rust's `CamZoomAction`
 *  (settings/model.rs): serde's externally-tagged encoding gives `{shrink:{to}}` for the struct
 *  variant and bare `"hide"`/`"stay"` for the unit ones. */
export type CamZoomAction = { shrink: { to: number } } | "hide" | "stay";
export interface ZoomSettings { enabled: boolean; target_scale: number; hold_ms: number; smoothness: number; clicks: number; camera_shrink: boolean; camera_shrink_min: number; smart_hold: boolean; smart_follow: boolean; cam_zoom_default?: CamZoomAction | null;
  /** Opt-in critically-damped smoothing on the auto-zoom CAMERA path (Rust `ZoomConfig::smoothing_ms`,
   *  `export/camera/smoothing.rs`) - NOT `CursorSettings.smoothness` (the cursor low-pass). 0 = off,
   *  bit-identical export. */
  camera_smoothing_ms: number }
export interface ClickFxSettings { enabled: boolean; style: ClickFxStyle; color: [number, number, number]; intensity: number; captions: boolean; spotlight: boolean; spotlight_dim: number; spotlight_radius: number; spotlight_feather: number; spotlight_mode: SpotlightMode; spotlight_tint: [number, number, number]; video_fx_mode: VideoFxMode; spotlight_dim_camera: boolean }
export interface HotkeySettings {
  zoom_hold: string; layout_screen: string; layout_camera: string;
  layout_presenter: string; layout_screen_only: string; layout_camera_only: string;
  spotlight_hold: string; video_fx_hold: string;
}
/** Which of `BackgroundSettings`' fields the renderer uses - mirrors Rust `settings::background::BackgroundKind`.
 *  `mesh` (default) is a bundled wallpaper image; `solid`/`gradient` are real user colors;
 *  `image`/`video` render the user's own imported file (`asset`), a GIF being a `video`. */
export type BackgroundKind = "mesh" | "solid" | "gradient" | "image" | "video";
export interface BackgroundSettings {
  kind: BackgroundKind;
  solid: [number, number, number];
  gradient_from: [number, number, number];
  gradient_to: [number, number, number];
  gradient_angle_deg: number;
  /** 0..1 softness applied once to the STATIC background buffer (cheap - rebuilt once per
   *  export/preview, not per frame). 0 = off (today's behavior). Because it is a one-off pass it
   *  reaches a video background's first frame only, so `BackgroundPanel` hides this slider while
   *  `kind === "video"` rather than showing a control that does nothing. */
  blur: number;
  /** Which bundled wallpaper `kind: "mesh"` renders (`settings::wallpapers::WALLPAPERS` id).
   *  EMPTY = the legacy "Classic" `bg.jpg`, which is what every project saved before the
   *  wallpaper library loads as, so those keep rendering byte-identically. */
  mesh: string;
  /** Optional middle stop for `kind: "gradient"`. Absent = the two-stop ramp, unchanged. */
  gradient_mid?: [number, number, number] | null;
  /** The user's imported background file, RELATIVE to the project folder (`background/<file>`),
   *  used by `kind: "image" | "video"`. Never absolute - projects stay portable. Kept when the
   *  user switches back to a wallpaper, so re-selecting the asset needs no re-import. */
  asset?: string | null;
  /** 0..0.8 black overlay over whichever background has pixels (wallpaper, image or video).
   *  Applied in Rust for everything the backend rasterises and in `stageBg.ts` for the moving
   *  preview branch only - exactly once either way, same formula. */
  dim: number;
}
export interface Settings {
  zoom: ZoomSettings; clickfx: ClickFxSettings; hotkeys: HotkeySettings; appearance: AppearanceSettings;
  cursor: CursorSettings; ui: InterfaceSettings; audio_offset_ms: number; background: BackgroundSettings;
  audio_mic_volume: number; audio_sys_volume: number; ai_model: string;
}
