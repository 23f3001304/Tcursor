pub fn rrect_sd(x: f32, y: f32, mn: [f32; 2], mx: [f32; 2], r: f32) -> f32 {
    let (cx, cy) = ((mn[0] + mx[0]) * 0.5, (mn[1] + mx[1]) * 0.5);
    let (hx, hy) = ((mx[0] - mn[0]) * 0.5 - r, (mx[1] - mn[1]) * 0.5 - r);
    let (qx, qy) = ((x - cx).abs() - hx, (y - cy).abs() - hy);
    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r
}

pub fn rrect_cov(x: f32, y: f32, mn: [f32; 2], mx: [f32; 2], r: f32) -> f32 {
    (0.5 - rrect_sd(x, y, mn, mx, r)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{rrect_cov, rrect_sd};

    const MN: [f32; 2] = [10.0, 20.0];
    const MX: [f32; 2] = [110.0, 60.0];

    #[test]
    fn the_distance_is_negative_inside_zero_on_the_edge_and_positive_outside() {
        assert!(
            rrect_sd(60.0, 40.0, MN, MX, 0.0) < -19.0,
            "the centre is deep inside"
        );
        assert!(
            rrect_sd(10.0, 40.0, MN, MX, 0.0).abs() < 1e-4,
            "the left edge is zero"
        );
        assert!(
            (rrect_sd(120.0, 40.0, MN, MX, 0.0) - 10.0).abs() < 1e-4,
            "ten px right of it"
        );
    }

    #[test]
    fn a_corner_radius_rounds_the_corner_in_and_nothing_else() {
        let square = rrect_sd(10.0, 20.0, MN, MX, 0.0);
        let round = rrect_sd(10.0, 20.0, MN, MX, 8.0);
        assert!(
            square.abs() < 1e-4 && round > 2.0,
            "the corner pulls inside: {square} {round}"
        );
        assert!(
            (rrect_sd(60.0, 20.0, MN, MX, 8.0)).abs() < 1e-4,
            "the middle of the top edge is untouched by the radius"
        );
    }

    #[test]
    fn coverage_is_one_inside_zero_outside_and_a_half_on_the_edge() {
        assert_eq!(rrect_cov(60.0, 40.0, MN, MX, 0.0), 1.0);
        assert_eq!(rrect_cov(200.0, 40.0, MN, MX, 0.0), 0.0);
        assert!((rrect_cov(10.0, 40.0, MN, MX, 0.0) - 0.5).abs() < 1e-4);
    }
}
