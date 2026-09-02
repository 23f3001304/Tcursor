// Split from pack.rs per repo convention (#[path] sibling test module).
use super::*;
use std::io::Write;

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
fn default_pack_id_resolves_to_builtin_sprites_verbatim_except_busy() {
    let rows = sprite_sources(DEFAULT_PACK_ID);
    assert_eq!(rows.len(), SPRITES.len());
    let arrow = SPRITES.iter().find(|&&(k, ..)| k == CursorType::Arrow).unwrap();
    for ((kind, bytes, hot), &(sk, spng, shot)) in rows.iter().zip(SPRITES.iter()) {
        assert_eq!(*kind, sk);
        if sk == CursorType::Busy {
            // Busy is remapped to Arrow's bytes/hotspot (busy_as_arrow) - not its own builtin PNG.
            assert_eq!(bytes.as_slice(), arrow.1);
            assert_eq!(*hot, arrow.2);
        } else {
            assert_eq!(bytes.as_slice(), spng);
            assert_eq!(*hot, shot);
        }
    }
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
fn list_cursor_packs_always_includes_the_builtin_default_first() {
    let packs = list_cursor_packs();
    assert_eq!(packs[0].id, DEFAULT_PACK_ID);
    assert!(packs[0].builtin);
}
