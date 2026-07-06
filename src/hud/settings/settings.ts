export type CamShape = "circle" | "rounded" | "rect";
export type CamCorner = "bottom_left" | "bottom_right" | "top_left" | "top_right";
export interface ModeAppearance {
  pad: number; screen_size: number; screen_radius: number; cam_size: number;
  cam_shape: CamShape; cam_radius: number; cam_corner: CamCorner;
  cam_margin_x: number; cam_margin_y: number;
}
export interface AppearanceSettings {
  screen: ModeAppearance; screen_only: ModeAppearance; camera: ModeAppearance;
  camera_only: ModeAppearance; presenter: ModeAppearance;
}
export type ThemeMode = "light" | "dark" | "system";
export interface InterfaceSettings { theme: ThemeMode; accent: [number, number, number] }
export type CursorStyle = "system" | "enhanced" | "hidden";
export interface CursorSettings { style: CursorStyle; size: number; motion_blur: number; click_bounce: boolean; bounce_intensity: number }
export type ClickFxStyle = "none" | "ripple" | "pulse" | "glow" | "shockwave" | "particles" | "neon";
export type SpotlightMode = "classic" | "blur" | "halo" | "breathing" | "nebula" | "vignette";
export type VideoFxMode = "nebulawash" | "cinematicdim" | "screenfocus" | "colorpop";
export interface ZoomSettings { enabled: boolean; target_scale: number; hold_ms: number; smoothness: number; clicks: number; camera_shrink: boolean; camera_shrink_min: number; smart_hold: boolean; smart_follow: boolean }
export interface ClickFxSettings { enabled: boolean; style: ClickFxStyle; color: [number, number, number]; intensity: number; captions: boolean; spotlight: boolean; spotlight_dim: number; spotlight_radius: number; spotlight_feather: number; spotlight_mode: SpotlightMode; spotlight_tint: [number, number, number]; video_fx_mode: VideoFxMode }
export interface HotkeySettings {
  zoom_hold: string; layout_screen: string; layout_camera: string;
  layout_presenter: string; layout_screen_only: string; layout_camera_only: string;
  spotlight_hold: string; video_fx_hold: string;
}
export interface Settings { zoom: ZoomSettings; clickfx: ClickFxSettings; hotkeys: HotkeySettings; appearance: AppearanceSettings; cursor: CursorSettings; ui: InterfaceSettings; audio_offset_ms: number }
