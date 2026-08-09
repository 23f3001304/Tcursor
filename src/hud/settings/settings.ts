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
export interface ZoomSettings { enabled: boolean; target_scale: number; hold_ms: number; smoothness: number; clicks: number; camera_shrink: boolean; camera_shrink_min: number; smart_hold: boolean; smart_follow: boolean; cam_zoom_default?: CamZoomAction | null }
export interface ClickFxSettings { enabled: boolean; style: ClickFxStyle; color: [number, number, number]; intensity: number; captions: boolean; spotlight: boolean; spotlight_dim: number; spotlight_radius: number; spotlight_feather: number; spotlight_mode: SpotlightMode; spotlight_tint: [number, number, number]; video_fx_mode: VideoFxMode; spotlight_dim_camera: boolean }
export interface HotkeySettings {
  zoom_hold: string; layout_screen: string; layout_camera: string;
  layout_presenter: string; layout_screen_only: string; layout_camera_only: string;
  spotlight_hold: string; video_fx_hold: string;
}
/** Which of `BackgroundSettings`' fields the renderer uses - mirrors Rust `settings::background::BackgroundKind`.
 *  `mesh` (default) is today's bundled image; `solid`/`gradient` are real user colors. Custom
 *  image/video backgrounds have no backend yet - `BackgroundPanel` flags them as "coming soon". */
export type BackgroundKind = "mesh" | "solid" | "gradient";
export interface BackgroundSettings {
  kind: BackgroundKind;
  solid: [number, number, number];
  gradient_from: [number, number, number];
  gradient_to: [number, number, number];
  gradient_angle_deg: number;
  /** 0..1 softness applied once to the static background buffer (cheap - rebuilt once per
   *  export/preview, not per frame). 0 = off (today's behavior). */
  blur: number;
}
export interface Settings {
  zoom: ZoomSettings; clickfx: ClickFxSettings; hotkeys: HotkeySettings; appearance: AppearanceSettings;
  cursor: CursorSettings; ui: InterfaceSettings; audio_offset_ms: number; background: BackgroundSettings;
  audio_mic_volume: number; audio_sys_volume: number; ai_model: string;
}
