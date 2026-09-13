// Split from pack.rs per repo convention (#[path] sibling test module).
use super::*;
use std::io::Write;
use std::path::PathBuf;
use crate::export::cursor::packlist::list_packs;

/// A minimal valid 1x1 PNG (smallest possible RGBA image), so tests can write real files
/// without needing an encoder - only byte presence/non-emptiness is exercised here.
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

fn temp_pack_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("tcursor_pack_test").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn busy_resolves_to_arrow_bytes_and_hotspot_for_the_builtin_pack() {
    let rows = sprite_sources(DEFAULT_PACK_ID);
    let (_, arrow_bytes, arrow_hot) = rows.iter().find(|(k, ..)| *k == CursorType::Arrow).unwrap();
    let (_, busy_bytes, busy_hot) = rows.iter().find(|(k, ..)| *k == CursorType::Busy).unwrap();
    assert_eq!(busy_bytes, arrow_bytes);
    assert_eq!(busy_hot, arrow_hot);
    // And it's NOT the raw builtin pinwheel asset anymore.
    let builtin_busy = SPRITES.iter().find(|&&(k, ..)| k == CursorType::Busy).unwrap();
    assert_ne!(busy_bytes.as_slice(), builtin_busy.1);
}

#[test]
fn busy_resolves_to_arrow_even_when_a_custom_pack_overrides_arrow() {
    let dir = temp_pack_dir("busy_follows_custom_arrow");
    std::fs::write(dir.join("arrow.png"), TINY_PNG).unwrap();
    let rows = sprite_sources_from_dir(&dir);
    // sprite_sources_from_dir alone does NOT apply busy_as_arrow (that's sprite_sources' job) -
    // confirm the raw builtin busy bytes still come back here, then confirm the public seam fixes it up.
    let (_, raw_busy_bytes, _) = rows.iter().find(|(k, ..)| *k == CursorType::Busy).unwrap();
    let builtin_busy = SPRITES.iter().find(|&&(k, ..)| k == CursorType::Busy).unwrap();
    assert_eq!(raw_busy_bytes.as_slice(), builtin_busy.1);

    let mut fixed = rows;
    busy_as_arrow(&mut fixed);
    let (_, busy_bytes, _) = fixed.iter().find(|(k, ..)| *k == CursorType::Busy).unwrap();
    assert_eq!(busy_bytes.as_slice(), TINY_PNG);
}

#[test]
fn empty_pack_id_also_resolves_to_builtin() {
    assert_eq!(sprite_sources("").len(), SPRITES.len());
}

#[test]
fn nonexistent_pack_folder_falls_back_to_builtin_for_every_kind() {
    let rows = sprite_sources_from_dir(Path::new("C:/tcursor_pack_that_does_not_exist"));
    assert_eq!(rows.len(), SPRITES.len());
    for (kind, bytes, hot) in &rows {
        let builtin = SPRITES.iter().find(|&&(k, ..)| k == *kind).unwrap();
        assert_eq!(bytes.as_slice(), builtin.1);
        assert_eq!(*hot, builtin.2);
    }
}

#[test]
fn custom_png_overrides_one_kind_others_fall_back_to_builtin() {
    let dir = temp_pack_dir("override_one");
    std::fs::write(dir.join("arrow.png"), TINY_PNG).unwrap();
    let mut hs = std::fs::File::create(dir.join("hotspots.json")).unwrap();
    hs.write_all(br#"{"arrow": [0.1, 0.2]}"#).unwrap();

    let rows = sprite_sources_from_dir(&dir);
    let (_, arrow_bytes, arrow_hot) = rows.iter().find(|(k, ..)| *k == CursorType::Arrow).unwrap();
    assert_eq!(arrow_bytes.as_slice(), TINY_PNG);
    assert_eq!(*arrow_hot, (0.1, 0.2));

    // Hand has no override file in this pack -> falls back to the builtin bytes + hotspot.
    let (_, hand_bytes, hand_hot) = rows.iter().find(|(k, ..)| *k == CursorType::Hand).unwrap();
    let builtin_hand = SPRITES.iter().find(|&&(k, ..)| k == CursorType::Hand).unwrap();
    assert_eq!(hand_bytes.as_slice(), builtin_hand.1);
    assert_eq!(*hand_hot, builtin_hand.2);
}

#[test]
fn missing_hotspot_entry_for_a_provided_kind_defaults_to_center() {
    let dir = temp_pack_dir("no_hotspot_entry");
    std::fs::write(dir.join("arrow.png"), TINY_PNG).unwrap();
    // No hotspots.json at all.
    let rows = sprite_sources_from_dir(&dir);
    let (_, _, hot) = rows.iter().find(|(k, ..)| *k == CursorType::Arrow).unwrap();
    assert_eq!(*hot, (0.5, 0.5));
}

#[test]
fn empty_png_file_is_treated_as_absent_and_falls_back() {
    let dir = temp_pack_dir("empty_png");
    std::fs::write(dir.join("arrow.png"), []).unwrap();
    let rows = sprite_sources_from_dir(&dir);
    let (_, bytes, _) = rows.iter().find(|(k, ..)| *k == CursorType::Arrow).unwrap();
    let builtin_arrow = SPRITES.iter().find(|&&(k, ..)| k == CursorType::Arrow).unwrap();
    assert_eq!(bytes.as_slice(), builtin_arrow.1);
}

#[test]
fn kind_filename_matches_serde_name_plus_png() {
    assert_eq!(kind_filename(CursorType::Arrow), "arrow.png");
    assert_eq!(kind_filename(CursorType::ResizeNs), "resize_ns.png");
}

#[test]
fn kind_name_round_trips_every_variant_through_serde() {
    for &(kind, ..) in SPRITES {
        let name = kind_name(kind);
        assert!(!name.is_empty());
        assert_eq!(kind_filename(kind), format!("{name}.png"));
    }
}

#[test]
fn a_v2_pack_declares_its_busy_animation_and_keeps_its_own_busy_sprite() {
    use crate::export::cursor::busy::BusyAnim;
    // "cat" ships `busy: { anim: "pulse" }` with a single busy.png and no explicit frames.
    let spec = busy_spec("cat").expect("the Cat pack declares a busy animation");
    assert_eq!((spec.anim, spec.frames), (BusyAnim::Pulse, 0));
    assert!(busy_frames("cat").is_empty(), "no busy_NN.png in this pack");
    // An animating pack opts OUT of the busy-is-arrow substitution: its busy bytes are its own.
    let rows = sprite_sources("cat");
    let (_, arrow, _) = rows.iter().find(|(k, ..)| *k == CursorType::Arrow).unwrap();
    let (_, busy, _) = rows.iter().find(|(k, ..)| *k == CursorType::Busy).unwrap();
    assert_ne!(busy, arrow, "an animated busy state keeps its own sprite");
}

#[test]
fn the_embedded_pack_and_a_v1_pack_declare_no_busy_animation() {
    // The last one is a folder with no pack.json (what `sprite_sources_from_dir` tolerates).
    for id in [DEFAULT_PACK_ID, "", "tcursor_pack_that_does_not_exist"] {
        assert!(busy_spec(id).is_none(), "{id}");
        assert!(busy_frames(id).is_empty(), "{id}");
    }
}

#[test]
fn every_shipped_pack_declares_a_usable_busy_animation() {
    for info in list_packs(None).iter().filter(|p| p.builtin && p.id != DEFAULT_PACK_ID) {
        let spec = busy_spec(&info.id).unwrap_or_else(|| panic!("{} declares no busy", info.id));
        assert!(spec.fps > 0.0, "{} has fps {}", info.id, spec.fps);
        // Every kind must resolve to real bytes, or the grid tile would render blank.
        let rows = sprite_sources(&info.id);
        assert_eq!(rows.len(), SPRITES.len(), "{}", info.id);
        assert!(rows.iter().all(|(_, b, _)| !b.is_empty()), "{}", info.id);
    }
}

/// THE EXPORT MUST NOT CHANGE. The embedded pack having a real folder on disk is a GRID
/// concern: `sprite_sources("default")` still returns `cursorset::SPRITES` verbatim (busy
/// remapped to arrow), never the folder's files. Pinned two ways - the bytes, and one actually
/// drawn frame, so a future "just resolve default through its folder" refactor cannot slip past.
#[test]
fn the_default_pack_still_exports_from_the_embedded_sprites_not_its_folder() {
    use crate::export::cursor::cursordraw::{decode_sprite, draw_cursor};

    let rows = sprite_sources(DEFAULT_PACK_ID);
    let arrow = SPRITES.iter().find(|&&(k, ..)| k == CursorType::Arrow).unwrap();
    for ((kind, bytes, hot), &(sk, spng, shot)) in rows.iter().zip(SPRITES.iter()) {
        assert_eq!(*kind, sk);
        let (want_png, want_hot) = if sk == CursorType::Busy { (arrow.1, arrow.2) } else { (spng, shot) };
        assert_eq!(bytes.as_slice(), want_png, "{sk:?} bytes came from somewhere else");
        assert_eq!(*hot, want_hot, "{sk:?} hotspot");
    }

    // And the pixels those bytes produce. `decode_sprite` shells out to ffmpeg, so this needs the
    // bundled binary - skip rather than fail where it is unavailable (same as other ffmpeg tests).
    let (_, png, hot) = rows.iter().find(|(k, ..)| *k == CursorType::Arrow).unwrap();
    let Some(spr) = decode_sprite(png, *hot) else { return };
    let mut drawn = vec![0u8; 64 * 64 * 4];
    draw_cursor(&mut drawn, 64, 64, &spr, (32.0, 32.0), &[], 24.0, 0.0, 1.0, (0, 0, 64, 64));
    let mut reference = vec![0u8; 64 * 64 * 4];
    let (_, embedded_png, embedded_hot) = SPRITES.iter().find(|&&(k, ..)| k == CursorType::Arrow).unwrap();
    let embedded = decode_sprite(embedded_png, *embedded_hot).unwrap();
    draw_cursor(&mut reference, 64, 64, &embedded, (32.0, 32.0), &[], 24.0, 0.0, 1.0, (0, 0, 64, 64));
    assert_eq!(drawn, reference, "a drawn Default frame must be byte-identical to the embedded one");
    assert!(drawn.iter().any(|&b| b != 0), "the frame actually drew something");
}

#[test]
fn only_the_embedded_default_set_inverts_for_a_dark_theme() {
    assert!(theme_inverts(DEFAULT_PACK_ID));
    for id in ["cartoon", "gradient-glass", "neon", "my-imported-pack"] {
        assert!(!theme_inverts(id), "{id} is artwork with its own colours");
    }
}
