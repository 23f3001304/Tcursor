#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    White,
    Accent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub fill: Fill,
    pub shadow: bool,
    pub plate: bool,
    pub plate_rgb: [u8; 3],
    pub plate_alpha: f32,
    pub rule: bool,
}

const CLEAN: TextStyle = TextStyle {
    fill: Fill::White,
    shadow: true,
    plate: false,
    plate_rgb: [0, 0, 0],
    plate_alpha: 0.0,
    rule: false,
};

pub fn style_of(name: &str) -> TextStyle {
    match name {
        "plate" => TextStyle {
            shadow: false,
            plate: true,
            plate_alpha: 0.62,
            ..CLEAN
        },
        "accent" => TextStyle {
            fill: Fill::Accent,
            ..CLEAN
        },
        "bar" => TextStyle {
            shadow: false,
            rule: true,
            ..CLEAN
        },
        _ => CLEAN,
    }
}

pub fn fill_rgb(f: Fill, accent: [u8; 3]) -> [u8; 3] {
    match f {
        Fill::White => [255, 255, 255],
        Fill::Accent => accent,
    }
}

#[cfg(test)]
#[path = "text_style_tests.rs"]
mod tests;
