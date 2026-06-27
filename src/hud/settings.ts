export type ClickFxStyle = "none" | "ripple" | "pulse";
export interface ZoomSettings { enabled: boolean; target_scale: number; hold_ms: number; smoothness: number; clicks: number }
export interface ClickFxSettings { enabled: boolean; style: ClickFxStyle; color: [number, number, number]; intensity: number; captions: boolean; spotlight: boolean }
export interface HotkeySettings {
  zoom_hold: string; layout_screen: string; layout_camera: string;
  layout_presenter: string; layout_screen_only: string; layout_camera_only: string;
}
export interface Settings { zoom: ZoomSettings; clickfx: ClickFxSettings; hotkeys: HotkeySettings; audio_offset_ms: number }
