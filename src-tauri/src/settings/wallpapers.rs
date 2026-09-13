// The curated background set: the bundled wallpaper images + 12 gradient presets.
//
// The images live in `assets/backgrounds/wallpapers/` (1536x864 JPEG, see the README beside
// them) and are embedded in the binary with `include_bytes!` - the same treatment `bg.jpg` (the
// legacy "Classic" mesh) already gets in `export::render`, so a wallpaper needs no install-time
// file layout and no network. They decode through `ffio::decode_image_cover`, the legacy mesh's
// own decode path with a cover fit, so a non-16:9 export crops the art rather than stretching it.
//
// `WALLPAPERS` itself is GENERATED: `build.rs` scans that folder and writes the table, so the set
// is whatever ships in `assets/` and no id can be hand-listed wrongly or forgotten. Ids, names and
// groups all come from the filenames - see `build.rs` for the rules.
//
// The gradients are pure math (`export::scene::background::render`), 2 or 3 stops at varied
// angles, curated for a screen-recording BACKDROP: the screen panel covers the middle, so
// nothing here is neon and nothing fights the content.

/// One bundled wallpaper: a stable id (what `BackgroundSettings.mesh` stores), a display name,
/// the section it belongs to in the picker, and the embedded JPEG.
pub struct Wallpaper { pub id: &'static str, pub name: &'static str, pub group: &'static str, pub bytes: &'static [u8] }

/// `0xRRGGBB` -> `[R, G, B]`, so the gradient table reads as the colours it is.
const fn c(hex: u32) -> [u8; 3] { [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8] }

/// One gradient preset: 2 stops, or 3 when `mid` is set.
pub struct GradientWallpaper {
    pub id: &'static str, pub name: &'static str,
    pub from: [u8; 3], pub mid: Option<[u8; 3]>, pub to: [u8; 3], pub angle_deg: f32,
}

// `WALLPAPERS_GEN` - the scanned table, written by `build.rs` from the asset folder.
include!(concat!(env!("OUT_DIR"), "/wallpapers_gen.rs"));

/// Every bundled wallpaper, in the order the picker shows them: Ribbons, then Folds, then Scenic,
/// alphabetical by name inside each group. Discovered at build time, never hand-listed.
///
/// The legacy `bg.jpg` ("Classic") is deliberately absent: the EMPTY id means "the legacy mesh",
/// so it is reached by `wallpaper_by_id`'s `None` arm rather than by a table entry. That keeps
/// every pre-library project rendering byte-identically with no special case here, and the editor
/// puts its tile first in the grid.
pub static WALLPAPERS: &[Wallpaper] = WALLPAPERS_GEN;

pub const GRADIENT_WALLPAPERS: &[GradientWallpaper] = &[
    GradientWallpaper { id: "indigo", name: "Indigo", from: c(0x070C22), mid: None, to: c(0x1B3A9E), angle_deg: 20.0 },
    GradientWallpaper { id: "tile", name: "Tile", from: c(0x0C1740), mid: Some(c(0x2445B8)), to: c(0x4C7BFF), angle_deg: 115.0 },
    GradientWallpaper { id: "dusk", name: "Dusk", from: c(0x1B2440), mid: Some(c(0x4A3A63)), to: c(0x8A5A68), angle_deg: 160.0 },
    GradientWallpaper { id: "ember", name: "Ember", from: c(0x2A1220), mid: None, to: c(0xA8433A), angle_deg: 45.0 },
    GradientWallpaper { id: "harbor", name: "Harbor", from: c(0x06202F), mid: None, to: c(0x12566E), angle_deg: 200.0 },
    GradientWallpaper { id: "moss", name: "Moss", from: c(0x16241B), mid: None, to: c(0x35573E), angle_deg: 135.0 },
    GradientWallpaper { id: "grape", name: "Grape", from: c(0x180E2C), mid: Some(c(0x35205A)), to: c(0x5B3A7A), angle_deg: 300.0 },
    GradientWallpaper { id: "slate", name: "Slate", from: c(0x14171D), mid: None, to: c(0x333B48), angle_deg: 180.0 },
    GradientWallpaper { id: "coral", name: "Coral", from: c(0x6B2B33), mid: Some(c(0xC56B52)), to: c(0xE8A87B), angle_deg: 60.0 },
    GradientWallpaper { id: "fog", name: "Fog", from: c(0xE9E7E2), mid: None, to: c(0xC4C1BA), angle_deg: 250.0 },
    GradientWallpaper { id: "blush", name: "Blush", from: c(0xF1E7E3), mid: Some(c(0xD9BFC0)), to: c(0xAE97A6), angle_deg: 330.0 },
    GradientWallpaper { id: "carbon", name: "Carbon", from: c(0x0B0C0F), mid: None, to: c(0x24272E), angle_deg: 90.0 },
];

pub fn wallpaper_by_id(id: &str) -> Option<&'static Wallpaper> { WALLPAPERS.iter().find(|w| w.id == id) }

#[cfg(test)]
#[path = "wallpapers_tests.rs"]
mod tests;
