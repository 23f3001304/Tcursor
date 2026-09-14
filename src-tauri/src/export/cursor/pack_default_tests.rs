// The embedded Default pack's own pins, split out of `pack_tests.rs` for the 200-line cap: the
// export must keep drawing it from the embedded sprites whatever its folder holds, and it is the
// only set that inverts for a dark theme.
use super::*;

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
