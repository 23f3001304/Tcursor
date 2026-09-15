use super::*;
use crate::settings::cursor::{CursorSettings, CursorStyle};

#[test]
fn defaults_match_tuned_zoom_and_round_trip() {
    let s = Settings::default();
    assert!(!s.clickfx.spotlight);
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
    assert_eq!(cfg.zoom_in_ms, 350);
    assert_eq!(s.zoom.clicks, 1);
    assert_eq!(cfg.clicks_to_trigger, 1);
    let json = serde_json::to_string(&s).unwrap();
    let back: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(back, s);
}
#[test]
fn partial_json_fills_defaults() {
    let back: Settings = serde_json::from_str("{\"zoom\":{\"enabled\":false}}").unwrap_or_default();
    assert!(!back.zoom.enabled);
    assert_eq!(back.zoom.target_scale, 2.2);
    assert_eq!(back.clickfx.enabled, ClickFxSettings::default().enabled);
    assert_eq!(back.clickfx.spotlight_dim, 0.60);
    assert_eq!(back.clickfx.spotlight_radius, 0.13);
    assert_eq!(back.clickfx.spotlight_feather, 0.10);
    assert_eq!(back.clickfx.spotlight_mode, SpotlightMode::Classic);
    assert_eq!(back.clickfx.spotlight_tint, [130, 90, 255]);
    assert!(back.clickfx.spotlight_dim_camera);
    assert!(back.zoom.camera_shrink);
    assert_eq!(back.zoom.camera_shrink_min, 0.62);
    assert!(back.zoom.smart_hold);
    assert_eq!(
        back.appearance,
        crate::settings::appearance::AppearanceSettings::default()
    );
    assert_eq!(
        back.cursor,
        crate::settings::cursor::CursorSettings::default()
    );
    assert_eq!(back.cursor.style, CursorStyle::System);
    assert_eq!(back.cursor.pack, "default");
    assert_eq!(CursorSettings::default().bounce_intensity, 0.5);
    assert!(CursorStyle::System.captures_os_cursor());
    assert!(!CursorStyle::Enhanced.captures_os_cursor());
    assert!(!CursorStyle::Hidden.captures_os_cursor());
    assert_eq!(
        back.ui,
        crate::settings::model::InterfaceSettings::default()
    );
    assert_eq!(back.ui.theme, ThemeMode::Light);
    assert_eq!(back.ui.accent, [239, 68, 68]);
    assert!(back.ui.interface_effects);
    assert_eq!(
        back.background,
        crate::settings::background::BackgroundSettings::default()
    );
    assert_eq!(
        back.background.kind,
        crate::settings::background::BackgroundKind::Mesh
    );
    assert_eq!(back.audio_mic_volume, 1.0);
    assert_eq!(back.audio_sys_volume, 1.0);
    assert_eq!(back.ai_model, "");
    assert_eq!(back.zoom.camera_smoothing_ms, 0);
    assert!(back.layout_presets.is_empty());
}

#[test]
fn layout_presets_round_trip_and_default_empty() {
    let back: Settings = serde_json::from_str("{}").unwrap();
    assert!(back.layout_presets.is_empty());
    assert!(Settings::default().layout_presets.is_empty());

    let mut look = crate::settings::appearance::AppearanceSettings::default();
    look.presenter.pad = 0.075;
    look.screen.cam_ring = Some(crate::settings::appearance::CamRing {
        width: 0.04,
        color: [1, 2, 3],
    });
    let mut s = Settings::default();
    s.layout_presets = vec![LayoutPreset {
        id: "p1".into(),
        name: "Bold".into(),
        appearance: look.clone(),
    }];
    let json = serde_json::to_string(&s).unwrap();
    let back: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(back, s);
    assert_eq!(back.layout_presets[0].name, "Bold");
    assert_eq!(back.layout_presets[0].appearance, look);
    assert_eq!(back.layout_presets[0].appearance.presenter.pad, 0.075);
}

#[test]
fn interface_effects_defaults_on_and_round_trips() {
    assert!(InterfaceSettings::default().interface_effects);
    let ui: InterfaceSettings = serde_json::from_str("{\"theme\":\"dark\"}").unwrap();
    assert_eq!(ui.theme, ThemeMode::Dark);
    assert!(ui.interface_effects);
    let mut s = Settings::default();
    s.ui.interface_effects = false;
    let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert!(!back.ui.interface_effects);
    assert_eq!(back, s);
}

#[test]
fn camera_smoothing_ms_round_trips_into_zoom_config() {
    let z: ZoomSettings = serde_json::from_str("{}").unwrap();
    assert_eq!(z.camera_smoothing_ms, 0);
    assert_eq!(z.to_zoom_config().smoothing_ms, 0);

    let z = ZoomSettings {
        camera_smoothing_ms: 120,
        ..ZoomSettings::default()
    };
    assert_eq!(z.to_zoom_config().smoothing_ms, 120);
}

#[test]
fn the_pointer_replay_is_off_until_the_user_asks_for_it() {
    assert!(!InterfaceSettings::default().ai_choreography);
}

#[test]
fn a_config_written_before_this_setting_existed_loads_with_it_off() {
    let s: InterfaceSettings =
        serde_json::from_str(r#"{"theme":"dark","accent":[1,2,3],"animated_brand":true}"#).unwrap();
    assert!(!s.ai_choreography);
}
