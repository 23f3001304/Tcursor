use super::settings;
use crate::export::grade::{apply_px, params_of, vignette_k, GradeParams};
use crate::settings::grade::GradePreset;

const COLOURS: [[u8; 3]; 12] = [
    [0, 0, 0],
    [255, 255, 255],
    [128, 128, 128],
    [255, 0, 0],
    [0, 255, 0],
    [0, 0, 255],
    [255, 255, 0],
    [0, 255, 255],
    [255, 0, 255],
    [224, 172, 148],
    [18, 24, 48],
    [239, 68, 68],
];

const PRESETS: [GradePreset; 9] = [
    GradePreset::None,
    GradePreset::Cinematic,
    GradePreset::Noir,
    GradePreset::Vintage,
    GradePreset::Frost,
    GradePreset::Golden,
    GradePreset::Midnight,
    GradePreset::Vivid,
    GradePreset::Dreamy,
];

fn graded(p: GradePreset, c: [u8; 3], u: f32, v: f32) -> [u8; 3] {
    let s = settings(p);
    let params = params_of(&s).unwrap_or(GradeParams {
        exposure: 0.0,
        contrast: 1.0,
        vignette: 0.0,
        saturation: 1.0,
        temp: 0.0,
        tint: 0.0,
        lift: [0.0; 3],
        gamma: [1.0; 3],
        gain: [1.0; 3],
    });
    let f = apply_px(
        [
            c[0] as f32 / 255.0,
            c[1] as f32 / 255.0,
            c[2] as f32 / 255.0,
        ],
        &params,
        u,
        v,
    );
    [
        (f[0] * 255.0).round() as u8,
        (f[1] * 255.0).round() as u8,
        (f[2] * 255.0).round() as u8,
    ]
}

const CENTRE: [[u8; 3]; 108] = [
    [0, 0, 0],       // None [0, 0, 0]
    [255, 255, 255], // None [255, 255, 255]
    [128, 128, 128], // None [128, 128, 128]
    [255, 0, 0],     // None [255, 0, 0]
    [0, 255, 0],     // None [0, 255, 0]
    [0, 0, 255],     // None [0, 0, 255]
    [255, 255, 0],   // None [255, 255, 0]
    [0, 255, 255],   // None [0, 255, 255]
    [255, 0, 255],   // None [255, 0, 255]
    [224, 172, 148], // None [224, 172, 148]
    [18, 24, 48],    // None [18, 24, 48]
    [239, 68, 68],   // None [239, 68, 68]
    [0, 0, 0],       // Cinematic [0, 0, 0]
    [255, 255, 255], // Cinematic [255, 255, 255]
    [127, 129, 133], // Cinematic [128, 128, 128]
    [247, 0, 0],     // Cinematic [255, 0, 0]
    [4, 255, 10],    // Cinematic [0, 255, 0]
    [0, 0, 244],     // Cinematic [0, 0, 255]
    [255, 255, 15],  // Cinematic [255, 255, 0]
    [6, 255, 255],   // Cinematic [0, 255, 255]
    [249, 0, 249],   // Cinematic [255, 0, 255]
    [228, 178, 157], // Cinematic [224, 172, 148]
    [8, 16, 45],     // Cinematic [18, 24, 48]
    [236, 67, 72],   // Cinematic [239, 68, 68]
    [0, 0, 0],       // Noir [0, 0, 0]
    [255, 255, 255], // Noir [255, 255, 255]
    [134, 134, 134], // Noir [128, 128, 128]
    [34, 34, 34],    // Noir [255, 0, 0]
    [200, 200, 200], // Noir [0, 255, 0]
    [0, 0, 0],       // Noir [0, 0, 255]
    [255, 255, 255], // Noir [255, 255, 0]
    [223, 223, 223], // Noir [0, 255, 255]
    [58, 58, 58],    // Noir [255, 0, 255]
    [209, 209, 209], // Noir [224, 172, 148]
    [0, 0, 0],       // Noir [18, 24, 48]
    [103, 103, 103], // Noir [239, 68, 68]
    [22, 19, 16],    // Vintage [0, 0, 0]
    [244, 240, 225], // Vintage [255, 255, 255]
    [137, 131, 120], // Vintage [128, 128, 128]
    [206, 29, 26],   // Vintage [255, 0, 0]
    [57, 227, 50],   // Vintage [0, 255, 0]
    [25, 22, 180],   // Vintage [0, 0, 255]
    [240, 237, 61],  // Vintage [255, 255, 0]
    [60, 230, 215],  // Vintage [0, 255, 255]
    [209, 33, 190],  // Vintage [255, 0, 255]
    [212, 171, 143], // Vintage [224, 172, 148]
    [40, 41, 50],    // Vintage [18, 24, 48]
    [208, 86, 78],   // Vintage [239, 68, 68]
    [0, 0, 1],       // Frost [0, 0, 0]
    [251, 255, 255], // Frost [255, 255, 255]
    [124, 135, 149], // Frost [128, 128, 128]
    [222, 5, 9],     // Frost [255, 0, 0]
    [22, 252, 28],   // Frost [0, 255, 0]
    [0, 0, 228],     // Frost [0, 0, 255]
    [249, 255, 35],  // Frost [255, 255, 0]
    [25, 255, 255],  // Frost [0, 255, 255]
    [225, 8, 236],   // Frost [255, 0, 255]
    [215, 183, 176], // Frost [224, 172, 148]
    [15, 24, 55],    // Frost [18, 24, 48]
    [216, 76, 87],   // Frost [239, 68, 68]
    [0, 0, 0],       // Golden [0, 0, 0]
    [255, 255, 242], // Golden [255, 255, 255]
    [154, 139, 114], // Golden [128, 128, 128]
    [255, 0, 0],     // Golden [255, 0, 0]
    [0, 255, 0],     // Golden [0, 255, 0]
    [0, 0, 255],     // Golden [0, 0, 255]
    [255, 255, 0],   // Golden [255, 255, 0]
    [0, 255, 245],   // Golden [0, 255, 255]
    [255, 0, 253],   // Golden [255, 0, 255]
    [255, 187, 131], // Golden [224, 172, 148]
    [21, 23, 38],    // Golden [18, 24, 48]
    [255, 70, 53],   // Golden [239, 68, 68]
    [0, 0, 0],       // Midnight [0, 0, 0]
    [228, 242, 255], // Midnight [255, 255, 255]
    [100, 109, 134], // Midnight [128, 128, 128]
    [194, 0, 0],     // Midnight [255, 0, 0]
    [7, 230, 17],    // Midnight [0, 255, 0]
    [0, 0, 233],     // Midnight [0, 0, 255]
    [224, 238, 25],  // Midnight [255, 255, 0]
    [11, 233, 255],  // Midnight [0, 255, 255]
    [197, 0, 242],   // Midnight [255, 0, 255]
    [189, 156, 162], // Midnight [224, 172, 148]
    [0, 2, 40],      // Midnight [18, 24, 48]
    [189, 52, 73],   // Midnight [239, 68, 68]
    [0, 0, 0],       // Vivid [0, 0, 0]
    [255, 255, 255], // Vivid [255, 255, 255]
    [133, 132, 131], // Vivid [128, 128, 128]
    [255, 0, 0],     // Vivid [255, 0, 0]
    [0, 255, 0],     // Vivid [0, 255, 0]
    [0, 0, 255],     // Vivid [0, 0, 255]
    [255, 255, 0],   // Vivid [255, 255, 0]
    [0, 255, 255],   // Vivid [0, 255, 255]
    [255, 0, 255],   // Vivid [255, 0, 255]
    [255, 181, 144], // Vivid [224, 172, 148]
    [1, 10, 46],     // Vivid [18, 24, 48]
    [255, 50, 49],   // Vivid [239, 68, 68]
    [30, 28, 31],    // Dreamy [0, 0, 0]
    [240, 240, 240], // Dreamy [255, 255, 255]
    [145, 143, 145], // Dreamy [128, 128, 128]
    [255, 24, 27],   // Dreamy [255, 0, 0]
    [15, 246, 16],   // Dreamy [0, 255, 0]
    [28, 27, 255],   // Dreamy [0, 0, 255]
    [241, 241, 12],  // Dreamy [255, 255, 0]
    [13, 244, 244],  // Dreamy [0, 255, 255]
    [255, 22, 255],  // Dreamy [255, 0, 255]
    [231, 180, 159], // Dreamy [224, 172, 148]
    [47, 51, 78],    // Dreamy [18, 24, 48]
    [251, 87, 90],   // Dreamy [239, 68, 68]
];

const POSITIONS: [[u8; 3]; 18] = [
    [228, 242, 255], // Midnight [255, 255, 255] at (0, 0)
    [100, 109, 134], // Midnight [128, 128, 128] at (0, 0)
    [189, 156, 162], // Midnight [224, 172, 148] at (0, 0)
    [207, 219, 247], // Midnight [255, 255, 255] at (0.5, 0)
    [91, 99, 122],   // Midnight [128, 128, 128] at (0.5, 0)
    [172, 142, 147], // Midnight [224, 172, 148] at (0.5, 0)
    [132, 140, 158], // Midnight [255, 255, 255] at (0.5, 0.5)
    [58, 63, 78],    // Midnight [128, 128, 128] at (0.5, 0.5)
    [110, 90, 94],   // Midnight [224, 172, 148] at (0.5, 0.5)
    [255, 255, 255], // Noir [255, 255, 255] at (0, 0)
    [134, 134, 134], // Noir [128, 128, 128] at (0, 0)
    [209, 209, 209], // Noir [224, 172, 148] at (0, 0)
    [255, 255, 255], // Noir [255, 255, 255] at (0.5, 0)
    [123, 123, 123], // Noir [128, 128, 128] at (0.5, 0)
    [190, 190, 190], // Noir [224, 172, 148] at (0.5, 0)
    [176, 176, 176], // Noir [255, 255, 255] at (0.5, 0.5)
    [81, 81, 81],    // Noir [128, 128, 128] at (0.5, 0.5)
    [125, 125, 125], // Noir [224, 172, 148] at (0.5, 0.5)
];

#[test]
fn the_grade_is_pinned_at_the_frame_centre_for_every_preset_and_colour() {
    let mut i = 0;
    for p in PRESETS {
        for c in COLOURS {
            let got = graded(p, c, 0.0, 0.0);
            assert_eq!(got, CENTRE[i], "{p:?} on {c:?} at the centre (row {i})");
            i += 1;
        }
    }
    assert_eq!(i, 108);
}

#[test]
fn the_vignette_darkens_an_edge_and_a_corner_more_than_the_centre() {
    let heavy = [GradePreset::Midnight, GradePreset::Noir];
    let uv = [(0.0f32, 0.0f32), (0.5, 0.0), (0.5, 0.5)];
    let cols: [[u8; 3]; 3] = [[255, 255, 255], [128, 128, 128], [224, 172, 148]];
    let mut i = 0;
    for p in heavy {
        for (u, v) in uv {
            for c in cols {
                assert_eq!(
                    graded(p, c, u, v),
                    POSITIONS[i],
                    "{p:?} {c:?} at ({u}, {v}) row {i}"
                );
                i += 1;
            }
        }
    }
    assert_eq!(i, 18);
    assert!(
        vignette_k(0.0, 0.0) == 0.0 && vignette_k(0.5, 0.5) == 1.0,
        "0 at the centre, 1 at a corner"
    );
}

#[test]
#[ignore = "generator: run once, paste into CENTRE"]
fn print_grade_table() {
    for p in PRESETS {
        for c in COLOURS {
            let g = graded(p, c, 0.0, 0.0);
            println!("[{}, {}, {}], // {p:?} {c:?}", g[0], g[1], g[2]);
        }
    }
}

#[test]
#[ignore = "generator: run once, paste into POSITIONS"]
fn print_grade_positions() {
    for p in [GradePreset::Midnight, GradePreset::Noir] {
        for (u, v) in [(0.0f32, 0.0f32), (0.5, 0.0), (0.5, 0.5)] {
            for c in [[255u8, 255, 255], [128, 128, 128], [224, 172, 148]] {
                let g = graded(p, c, u, v);
                println!(
                    "[{}, {}, {}], // {p:?} {c:?} at ({u}, {v})",
                    g[0], g[1], g[2]
                );
            }
        }
    }
}
