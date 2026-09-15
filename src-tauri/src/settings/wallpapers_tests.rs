use super::*;

const SOI: [u8; 2] = [0xFF, 0xD8];

const GROUPS: [&str; 5] = ["Ribbons", "Folds", "Gradients", "Metal", "Scenic"];

#[test]
fn the_generated_table_is_populated_with_unique_ids() {
    assert!(
        WALLPAPERS.len() >= 12,
        "only {} wallpapers - did the build-script scan run?",
        WALLPAPERS.len()
    );
    let mut ids: Vec<_> = WALLPAPERS.iter().map(|w| w.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        WALLPAPERS.len(),
        "wallpaper ids must be unique - `BackgroundSettings.mesh` stores one"
    );
    assert_eq!(GRADIENT_WALLPAPERS.len(), 12);
    let mut gids: Vec<_> = GRADIENT_WALLPAPERS.iter().map(|g| g.id).collect();
    gids.sort_unstable();
    gids.dedup();
    assert_eq!(gids.len(), 12, "gradient ids must be unique");
}

#[test]
fn every_wallpaper_embeds_a_real_jpeg_a_name_and_a_known_group() {
    for w in WALLPAPERS {
        assert_eq!(&w.bytes[..2], &SOI, "{}: not a JPEG", w.id);
        assert!(
            w.bytes.len() > 10_000,
            "{}: {} bytes, suspiciously small",
            w.id,
            w.bytes.len()
        );
        assert!(
            GROUPS.contains(&w.group),
            "{}: unknown group {:?}",
            w.id,
            w.group
        );
        assert!(
            !w.name.is_empty() && w.name.starts_with(|c: char| c.is_uppercase()),
            "{}: bad name {:?}",
            w.id,
            w.name
        );
        assert!(
            !w.name.contains('-'),
            "{}: hyphens should have become spaces, got {:?}",
            w.id,
            w.name
        );
    }
}

#[test]
fn the_table_is_grouped_in_picker_order_and_alphabetical_inside_each_group() {
    let mut last = (0usize, "");
    for w in WALLPAPERS {
        let g = GROUPS
            .iter()
            .position(|x| *x == w.group)
            .expect("known group");
        assert!(
            g >= last.0,
            "{}: group {:?} came after {:?}",
            w.id,
            w.group,
            GROUPS[last.0]
        );
        if g == last.0 {
            assert!(
                w.name >= last.1,
                "{:?} sorts before {:?} inside {}",
                w.name,
                last.1,
                w.group
            );
        }
        last = (g, w.name);
    }
}

#[test]
fn the_naming_rules_survived_the_build_script() {
    const PREFIXES: [(&str, &str); 4] = [
        ("folds", "Folds"),
        ("gradient", "Gradients"),
        ("metal", "Metal"),
        ("scenic", "Scenic"),
    ];
    for w in WALLPAPERS {
        let prefixed = w.id.split_once('-').and_then(|(p, rest)| {
            PREFIXES
                .iter()
                .find(|(x, _)| *x == p)
                .map(|(_, g)| (*g, rest))
        });
        match prefixed {
            Some((group, rest)) => {
                assert_eq!(w.group, group, "{}: wrong group for its prefix", w.id);
                assert_eq!(
                    w.name.to_lowercase().replace(' ', "-"),
                    rest,
                    "{}: name must be the stem minus its prefix",
                    w.id
                );
            }
            None => assert_eq!(
                w.group, "Ribbons",
                "{}: an unprefixed file is a Ribbon",
                w.id
            ),
        }
    }
    assert!(
        WALLPAPERS.iter().any(|w| w.group == "Ribbons"),
        "the unprefixed files are the Ribbons group"
    );
    assert!(
        WALLPAPERS.iter().any(|w| w.group == "Folds"),
        "the folds- set should be bundled"
    );
}

#[test]
fn no_two_wallpapers_are_the_same_image() {
    for (i, a) in WALLPAPERS.iter().enumerate() {
        for b in &WALLPAPERS[i + 1..] {
            assert!(
                a.bytes != b.bytes,
                "{} and {} are the same image",
                a.id,
                b.id
            );
        }
    }
}

#[test]
fn gradient_angles_are_varied() {
    let mut angles: Vec<_> = GRADIENT_WALLPAPERS
        .iter()
        .map(|g| g.angle_deg as i32)
        .collect();
    angles.sort_unstable();
    angles.dedup();
    assert!(
        angles.len() >= 8,
        "only {} distinct angles - vary them",
        angles.len()
    );
    assert!(
        GRADIENT_WALLPAPERS.iter().any(|g| g.mid.is_some()),
        "some presets should be 3-stop"
    );
    assert!(GRADIENT_WALLPAPERS
        .iter()
        .all(|g| (0.0..360.0).contains(&g.angle_deg)));
}

#[test]
fn lookup_finds_a_known_id_and_rejects_an_unknown_one() {
    assert_eq!(
        wallpaper_by_id(WALLPAPERS[0].id).map(|w| w.name),
        Some(WALLPAPERS[0].name)
    );
    assert!(wallpaper_by_id("nope").is_none());
    assert!(
        wallpaper_by_id("").is_none(),
        "empty = the legacy mesh, never a library hit"
    );
}
