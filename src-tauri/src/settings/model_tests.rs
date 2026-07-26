use super::*;

#[test]
fn defaults_match_tuned_zoom_and_round_trip() {
    let s = Settings::default();
    assert!(s.zoom.enabled);
    assert_eq!(s.zoom.target_scale, 2.2);
    assert_eq!(s.zoom.hold_ms, 2200);
    assert!(s.zoom.camera_shrink);
    assert_eq!(s.zoom.camera_shrink_min, 0.62);
    assert!(s.zoom.smart_hold);
    let cfg = s.zoom.to_zoom_config();
    assert_eq!(cfg.target_scale, 2.2);
    assert_eq!(cfg.idle_release_ms, 2200);
    assert_eq!(cfg.follow_damping, 0.10);
    assert_eq!(cfg.zoom_in_ms, 350); // untouched ZoomConfig default
    assert_eq!(s.zoom.clicks, 1);
    assert_eq!(cfg.clicks_to_trigger, 1);
    let json = serde_json::to_string(&s).unwrap();
    let back: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(back, s);
}
#[test]
fn partial_json_fills_defaults() {
    let back: Settings = serde_json::from_str("{\"zoom\":{\"enabled\":false}}").unwrap_or_default();
    // missing fields fall back to defaults
    assert!(!back.zoom.enabled);
    assert_eq!(back.zoom.target_scale, 2.2);
    assert_eq!(back.clickfx.enabled, ClickFxSettings::default().enabled);
    // old JSON without spotlight_dim/radius/feather loads with defaults
    assert_eq!(back.clickfx.spotlight_dim, 0.60);
    assert_eq!(back.clickfx.spotlight_radius, 0.13);
    assert_eq!(back.clickfx.spotlight_feather, 0.10);
    assert_eq!(back.clickfx.spotlight_mode, SpotlightMode::Classic);
    assert_eq!(back.clickfx.spotlight_tint, [130, 90, 255]);
    // old JSON without spotlight_dim_camera loads with default true (today's dim-everything look)
    assert!(back.clickfx.spotlight_dim_camera);
    // old JSON without camera_shrink/camera_shrink_min loads with defaults
    assert!(back.zoom.camera_shrink);
    assert_eq!(back.zoom.camera_shrink_min, 0.62);
    // old JSON without smart_hold loads with default
    assert!(back.zoom.smart_hold);
    // old JSON without appearance loads the per-mode defaults
    assert_eq!(back.appearance, crate::settings::appearance::AppearanceSettings::default());
    // old JSON without cursor loads the System default (cursor stays baked-in)
    assert_eq!(back.cursor, crate::settings::model::CursorSettings::default());
    assert_eq!(back.cursor.style, CursorStyle::System);
    assert_eq!(back.cursor.pack, "default"); // old JSON without cursor.pack loads the built-in id
    assert_eq!(CursorSettings::default().bounce_intensity, 0.5);
    assert!(CursorStyle::System.captures_os_cursor());
    assert!(!CursorStyle::Enhanced.captures_os_cursor());
    assert!(!CursorStyle::Hidden.captures_os_cursor());
    // old JSON without ui loads the Light theme + red accent defaults
    assert_eq!(back.ui, crate::settings::model::InterfaceSettings::default());
    assert_eq!(back.ui.theme, ThemeMode::Light);
    assert_eq!(back.ui.accent, [239, 68, 68]);
    // old JSON without background loads the Mesh default (today's bg.jpg, byte-identical)
    assert_eq!(back.background, crate::settings::background::BackgroundSettings::default());
    assert_eq!(back.background.kind, crate::settings::background::BackgroundKind::Mesh);
    // old JSON without the new audio/AI fields loads full volume + no forced model
    assert_eq!(back.audio_mic_volume, 1.0);
    assert_eq!(back.audio_sys_volume, 1.0);
    assert_eq!(back.ai_model, "");
}
