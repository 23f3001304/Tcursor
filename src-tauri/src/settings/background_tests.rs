use super::*;

#[test]
fn default_is_the_legacy_mesh_and_round_trips() {
    let s = BackgroundSettings::default();
    assert_eq!(s.kind, BackgroundKind::Mesh);
    assert_eq!(s.blur, 0.0);
    assert!(
        s.mesh.is_empty(),
        "empty = the bundled bg.jpg, the pre-library look"
    );
    assert_eq!(s.gradient_mid, None);
    let json = serde_json::to_string(&s).unwrap();
    let back: BackgroundSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(back, s);
}

#[test]
fn partial_json_fills_defaults() {
    let back: BackgroundSettings =
        serde_json::from_str("{\"kind\":\"solid\",\"solid\":[1,2,3]}").unwrap();
    assert_eq!(back.kind, BackgroundKind::Solid);
    assert_eq!(back.solid, [1, 2, 3]);
    assert_eq!(back.gradient_angle_deg, 135.0);
}

#[test]
fn a_pre_wallpaper_doc_loads_as_the_legacy_bundled_image() {
    let json = "{\"kind\":\"mesh\",\"solid\":[24,24,30],\"gradient_from\":[36,41,56],\
        \"gradient_to\":[88,64,120],\"gradient_angle_deg\":135.0,\"blur\":0.0}";
    let back: BackgroundSettings = serde_json::from_str(json).unwrap();
    assert_eq!(back, BackgroundSettings::default());
}

#[test]
fn an_unset_middle_stop_is_not_serialized() {
    let json = serde_json::to_string(&BackgroundSettings::default()).unwrap();
    assert!(
        !json.contains("gradient_mid"),
        "an absent middle stop must not bloat every edit.json: {json}"
    );
    let three = BackgroundSettings {
        gradient_mid: Some([1, 2, 3]),
        ..Default::default()
    };
    assert!(serde_json::to_string(&three)
        .unwrap()
        .contains("gradient_mid"));
}

#[test]
fn an_asset_background_round_trips_and_pre_asset_docs_are_untouched() {
    let s = BackgroundSettings {
        kind: BackgroundKind::Video,
        asset: Some("background/loop.mp4".into()),
        dim: 0.4,
        ..Default::default()
    };
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(
        serde_json::from_str::<BackgroundSettings>(&json).unwrap(),
        s
    );
    let old: BackgroundSettings =
        serde_json::from_str("{\"kind\":\"mesh\",\"mesh\":\"\"}").unwrap();
    assert_eq!((old.asset, old.dim), (None, 0.0));
    assert!(!serde_json::to_string(&BackgroundSettings::default())
        .unwrap()
        .contains("asset"));
}

#[test]
fn dim_is_clamped_to_the_published_range() {
    let d = |v: f32| {
        BackgroundSettings {
            dim: v,
            ..Default::default()
        }
        .dim_clamped()
    };
    assert_eq!((d(-1.0), d(0.0), d(0.5), d(9.0)), (0.0, 0.0, 0.5, 0.8));
}

#[test]
fn the_two_new_kinds_serialise_lowercase() {
    assert_eq!(
        serde_json::to_string(&BackgroundKind::Image).unwrap(),
        "\"image\""
    );
    assert_eq!(
        serde_json::to_string(&BackgroundKind::Video).unwrap(),
        "\"video\""
    );
    let back: BackgroundSettings =
        serde_json::from_str("{\"kind\":\"image\",\"asset\":\"background/a.png\"}").unwrap();
    assert_eq!(back.kind, BackgroundKind::Image);
    assert_eq!(back.asset.as_deref(), Some("background/a.png"));
}
