pub struct Wallpaper {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub bytes: &'static [u8],
}

const fn c(hex: u32) -> [u8; 3] {
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8]
}

pub struct GradientWallpaper {
    pub id: &'static str,
    pub name: &'static str,
    pub from: [u8; 3],
    pub mid: Option<[u8; 3]>,
    pub to: [u8; 3],
    pub angle_deg: f32,
}

include!(concat!(env!("OUT_DIR"), "/wallpapers_gen.rs"));

pub static WALLPAPERS: &[Wallpaper] = WALLPAPERS_GEN;

pub const GRADIENT_WALLPAPERS: &[GradientWallpaper] = &[
    GradientWallpaper {
        id: "indigo",
        name: "Indigo",
        from: c(0x070C22),
        mid: None,
        to: c(0x1B3A9E),
        angle_deg: 20.0,
    },
    GradientWallpaper {
        id: "tile",
        name: "Tile",
        from: c(0x0C1740),
        mid: Some(c(0x2445B8)),
        to: c(0x4C7BFF),
        angle_deg: 115.0,
    },
    GradientWallpaper {
        id: "dusk",
        name: "Dusk",
        from: c(0x1B2440),
        mid: Some(c(0x4A3A63)),
        to: c(0x8A5A68),
        angle_deg: 160.0,
    },
    GradientWallpaper {
        id: "ember",
        name: "Ember",
        from: c(0x2A1220),
        mid: None,
        to: c(0xA8433A),
        angle_deg: 45.0,
    },
    GradientWallpaper {
        id: "harbor",
        name: "Harbor",
        from: c(0x06202F),
        mid: None,
        to: c(0x12566E),
        angle_deg: 200.0,
    },
    GradientWallpaper {
        id: "moss",
        name: "Moss",
        from: c(0x16241B),
        mid: None,
        to: c(0x35573E),
        angle_deg: 135.0,
    },
    GradientWallpaper {
        id: "grape",
        name: "Grape",
        from: c(0x180E2C),
        mid: Some(c(0x35205A)),
        to: c(0x5B3A7A),
        angle_deg: 300.0,
    },
    GradientWallpaper {
        id: "slate",
        name: "Slate",
        from: c(0x14171D),
        mid: None,
        to: c(0x333B48),
        angle_deg: 180.0,
    },
    GradientWallpaper {
        id: "coral",
        name: "Coral",
        from: c(0x6B2B33),
        mid: Some(c(0xC56B52)),
        to: c(0xE8A87B),
        angle_deg: 60.0,
    },
    GradientWallpaper {
        id: "fog",
        name: "Fog",
        from: c(0xE9E7E2),
        mid: None,
        to: c(0xC4C1BA),
        angle_deg: 250.0,
    },
    GradientWallpaper {
        id: "blush",
        name: "Blush",
        from: c(0xF1E7E3),
        mid: Some(c(0xD9BFC0)),
        to: c(0xAE97A6),
        angle_deg: 330.0,
    },
    GradientWallpaper {
        id: "carbon",
        name: "Carbon",
        from: c(0x0B0C0F),
        mid: None,
        to: c(0x24272E),
        angle_deg: 90.0,
    },
];

pub fn wallpaper_by_id(id: &str) -> Option<&'static Wallpaper> {
    WALLPAPERS.iter().find(|w| w.id == id)
}

#[cfg(test)]
#[path = "wallpapers_tests.rs"]
mod tests;
