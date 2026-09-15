fn main() {
    emit_wallpaper_table();
    tauri_build::build()
}

/// Write `$OUT_DIR/wallpapers_gen.rs`: the bundled wallpaper table, DISCOVERED from the asset
/// folder instead of hand-listed, so adding a wallpaper is just dropping a JPEG in.
/// `settings/wallpapers.rs` `include!`s the result and re-exports it as `WALLPAPERS`.
///
/// Everything comes from the filename: the id is the stem, the group is the `scenic-`/`folds-`
/// prefix (anything else is a Ribbon), and the name is the rest of the stem title-cased.
///
/// The emitted item is a private `const` rather than the public `static` itself, so the public
/// symbol (and its doc comment) still lives in a real source file where `docs-hover` can see it.
fn emit_wallpaper_table() {
    const DIR: &str = "assets/backgrounds/wallpapers";
    println!("cargo:rerun-if-changed={DIR}");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let mut found: Vec<(u8, String, &str, String)> = Vec::new(); // (group order, name, group, id)
    for entry in std::fs::read_dir(DIR).expect("read wallpapers dir") {
        let path = entry.expect("wallpaper dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("jpg") {
            continue;
        }
        let id = path
            .file_stem()
            .expect("stem")
            .to_string_lossy()
            .to_string();
        let (order, group, rest) = match id.split_once('-') {
            Some(("folds", rest)) => (1, "Folds", rest),
            Some(("gradient", rest)) => (2, "Gradients", rest),
            Some(("metal", rest)) => (3, "Metal", rest),
            Some(("scenic", rest)) => (4, "Scenic", rest),
            _ => (0, "Ribbons", id.as_str()),
        };
        found.push((order, title_case(rest), group, id.clone()));
    }
    // Ribbons, Folds, Gradients, Metal, Scenic; alphabetical by display name inside each group.
    // The `read_dir` order is arbitrary, so this sort is what makes the generated table stable -
    // and it is the order the editor's picker renders, one section per group.
    found.sort();
    let mut out = String::from("const WALLPAPERS_GEN: &[Wallpaper] = &[\n");
    for (_, name, group, id) in &found {
        // `include_bytes!` resolves relative to the file it appears in, which is this generated
        // one under OUT_DIR - so the path has to be absolute.
        let src = format!("{manifest}/{DIR}/{id}.jpg");
        out.push_str(&format!("    Wallpaper {{ id: {id:?}, name: {name:?}, group: {group:?}, bytes: include_bytes!({src:?}) }},\n"));
    }
    out.push_str("];\n");
    let dest =
        std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("wallpapers_gen.rs");
    std::fs::write(&dest, out).expect("write wallpapers_gen.rs");
}

/// `coast-dusk` -> `Coast Dusk`. Hyphens become spaces and each word is capitalized; anything
/// already uppercase is left alone.
fn title_case(stem: &str) -> String {
    stem.split('-')
        .map(|w| {
            let mut ch = w.chars();
            match ch.next() {
                Some(f) => f.to_uppercase().collect::<String>() + ch.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
