use super::*;
use crate::export::cursor::busy::BusyAnim;

#[test]
fn the_embedded_default_pack_lists_first_with_a_real_folder() {
    let packs = list_packs(None);
    let d = &packs[0];
    assert_eq!((d.id.as_str(), d.name.as_str(), d.builtin), (DEFAULT_PACK_ID, "Default", true));
    // It has a folder now (`assets/cursors`, shipped as a resource) - that is what lets its grid
    // tile show a sprite and cycle on hover like every other pack.
    assert!(!d.dir.is_empty(), "the embedded pack must resolve to a folder");
    assert!(Path::new(&d.dir).is_dir(), "{} is not a folder", d.dir);
}

#[test]
fn the_default_pack_resolves_all_nine_kinds_through_the_alias_map() {
    let d = &list_packs(None)[0];
    for &(kind, ..) in SPRITES {
        let name = kind_name(kind);
        let file = d.files.get(&name).unwrap_or_else(|| panic!("no file mapped for {name}"));
        assert!(Path::new(&d.dir).join(file).is_file(), "{name} -> {file} does not exist");
    }
    // The one legacy name: the embedded folder spells its arrow `pointer.png`.
    assert_eq!(d.files[&kind_name(CursorType::Arrow)], "pointer.png");
    assert_eq!(default_filename(CursorType::Arrow), "pointer.png");
    // Every other kind already matches the pack format's own filename.
    for &(kind, ..) in SPRITES {
        if kind != CursorType::Arrow {
            assert_eq!(default_filename(kind), kind_filename(kind), "{:?}", kind);
        }
    }
}

#[test]
fn a_still_busy_state_shows_the_arrow_in_the_grid_exactly_as_it_renders() {
    // `pack::busy_as_arrow` substitutes the arrow for Busy on any pack with no declared animation,
    // in the export AND the preview. The tile has to show the same thing or it would promise a
    // sprite (the embedded pack's beachball) that picking it never draws.
    let d = &list_packs(None)[0];
    assert!(d.busy.is_none(), "the embedded pack declares no busy animation");
    assert_eq!(d.files[&kind_name(CursorType::Busy)], d.files[&kind_name(CursorType::Arrow)]);
}

#[test]
fn a_v2_pack_keeps_its_own_busy_file_and_its_animation() {
    let packs = list_packs(None);
    let cat = packs.iter().find(|p| p.id == "cat").expect("the Cat pack should be listed");
    assert_eq!(cat.busy.map(|b| (b.anim, b.frames)), Some((BusyAnim::Pulse, 0)));
    assert_eq!(cat.files[&kind_name(CursorType::Busy)], "busy.png", "an animated pack keeps its own");
    assert_eq!(cat.files[&kind_name(CursorType::Arrow)], "arrow.png");
    assert!(cat.builtin && Path::new(&cat.dir).is_dir());
}

#[test]
fn a_kind_the_pack_does_not_ship_is_left_out_rather_than_pointing_at_nothing() {
    // The renderer falls back to the embedded sprite for a missing kind; the grid cannot show
    // `include_bytes!`, so it must simply skip that state instead of loading a 404.
    let dir = std::env::temp_dir().join(format!("tcursor-packlist-gap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("arrow.png"), b"x").unwrap();
    let files = pack_files(&dir, false, true);
    assert_eq!(files.len(), 1);
    assert_eq!(files[&kind_name(CursorType::Arrow)], "arrow.png");
    assert!(!files.contains_key(&kind_name(CursorType::Hand)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_packs_puts_the_embedded_default_first_then_every_bundled_pack() {
    let packs = list_packs(None);
    assert_eq!(packs[0].id, DEFAULT_PACK_ID);
    assert!(packs[0].builtin);
    // The fifteen shipped packs resolve from the test binary (see `packdirs_tests`) and are all
    // marked built in, whatever their own pack.json claims.
    let shipped: Vec<&CursorPackInfo> = packs[1..].iter().filter(|p| p.builtin).collect();
    assert!(shipped.len() >= 15, "expected the bundled packs, got {:?}", packs.len());
}

/// The sections the Cursor panel's picker curates, mirrored from `panels/packCategories.ts`.
const KNOWN_CATEGORIES: &[&str] = &["Classic", "Glass and glow", "Playful", "Drawn", "Retro", "Imported"];

#[test]
fn the_embedded_default_pack_is_listed_under_classic() {
    // It is the plain system arrow, and its category is stated HERE rather than in a pack.json,
    // because the embedded set is the one pack with no manifest of its own.
    assert_eq!(list_packs(None)[0].category, "Classic");
    assert_eq!(DEFAULT_PACK_CATEGORY, "Classic");
}

#[test]
fn every_bundled_pack_manifest_declares_a_known_category() {
    // Read the repo's OWN asset folder rather than whatever the test binary resolves as a
    // resource root, so this pins the manifests that actually ship. A pack with no category would
    // land in "Imported" beside the user's own imports, which is exactly the wrong shelf.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join("cursorpacks");
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).expect("the bundled cursorpacks folder") {
        let path = entry.expect("cursorpacks entry").path();
        if !path.is_dir() { continue; }
        let m = read_meta(&path).unwrap_or_else(|| panic!("{path:?} has no readable pack.json"));
        let category = m.category.unwrap_or_default();
        assert!(KNOWN_CATEGORIES.contains(&category.as_str()) && category != IMPORTED_CATEGORY,
            "{path:?} declares category {category:?}");
        seen += 1;
    }
    assert!(seen >= 15, "expected the fifteen bundled packs, found {seen}");
}

#[test]
fn a_pack_with_no_category_of_its_own_lists_under_imported() {
    // v1 is the only shape `pack_import` writes, so this is every pack the user brings in.
    let dir = std::env::temp_dir().join(format!("tcursor-packlist-cat-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("pack.json"), br#"{"id":"mine","name":"Mine"}"#).unwrap();
    assert_eq!(read_pack_meta(&dir, false).unwrap().category, "Imported");
    // A blank one is exactly as useless as no key at all, and trimming keeps "Playful " together
    // with "Playful" rather than opening a second section for it.
    assert_eq!(category_or_imported(Some("   ".to_string())), "Imported");
    assert_eq!(category_or_imported(None), "Imported");
    assert_eq!(category_or_imported(Some(" Playful ".to_string())), "Playful");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_freshly_imported_pack_gets_the_same_category_a_relisting_would_give_it() {
    let info = imported_info("mine".into(), "Mine".into(), Path::new("C:/nope"));
    assert_eq!(info.category, IMPORTED_CATEGORY);
    assert!(!info.builtin);
}

#[test]
fn one_id_is_one_row_even_if_an_import_shares_a_bundled_name() {
    // Bundled packs are listed first, and a later folder claiming the same id is dropped - the
    // grid would otherwise show two rows that both resolve to the bundled folder.
    let packs = list_packs(None);
    let mut ids: Vec<&str> = packs.iter().map(|p| p.id.as_str()).collect();
    ids.sort();
    let (before, _) = (ids.len(), ids.dedup());
    assert_eq!(ids.len(), before, "duplicate pack ids in the listing");
}
