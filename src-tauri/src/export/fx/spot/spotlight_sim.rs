use crate::edit::model::{EffectKind, EffectRegion};
use crate::export::camera::ease;
use crate::export::types::Easing;
use crate::settings::model::{ClickFxSettings, SpotlightMode};

pub(crate) fn region_alpha(e: &EffectRegion, et: u32) -> f32 {
    if et < e.start_ms || et >= e.end_ms {
        return 0.0;
    }
    let inn = (et - e.start_ms) as f32 / e.fade_in_ms.max(1) as f32;
    let outn = (e.end_ms - et) as f32 / e.fade_out_ms.max(1) as f32;
    inn.min(outn).clamp(0.0, 1.0)
}

struct SpotTransition {
    from_alpha: f32,
    start_ms: u32,
    dur_ms: u32,
}

#[derive(Default)]
pub struct SpotlightSim {
    driver: Option<usize>,
    transition: Option<SpotTransition>,
    alpha: f32,
}

impl SpotlightSim {
    pub fn new() -> Self {
        Self::default()
    }

    fn winner(effects: &[EffectRegion], et: u32) -> Option<usize> {
        effects
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                matches!(e.kind, EffectKind::Spotlight) && et >= e.start_ms && et < e.end_ms
            })
            .max_by_key(|(i, e)| (e.layer, *i))
            .map(|(i, _)| i)
    }

    pub fn resolve(&mut self, effects: &[EffectRegion], et: u32, settings_on: bool) -> f32 {
        let winner_idx = Self::winner(effects, et);
        if winner_idx != self.driver {
            if self.driver.is_some() {
                let dur_ms = match winner_idx {
                    Some(i) => effects[i].fade_in_ms,
                    None => effects[self.driver.unwrap()].fade_out_ms,
                };
                self.transition = Some(SpotTransition {
                    from_alpha: self.alpha,
                    start_ms: et,
                    dur_ms: dur_ms.max(1),
                });
            }
            self.driver = winner_idx;
        }
        let natural = winner_idx
            .map(|i| region_alpha(&effects[i], et))
            .unwrap_or(0.0);
        self.alpha = if let Some(tr) = &self.transition {
            let elapsed = et.saturating_sub(tr.start_ms);
            if elapsed < tr.dur_ms {
                let e = ease(Easing::Smooth, elapsed as f32 / tr.dur_ms as f32);
                tr.from_alpha + (natural - tr.from_alpha) * e
            } else {
                self.transition = None;
                natural
            }
        } else {
            natural
        };
        self.alpha.max(if settings_on { 1.0 } else { 0.0 })
    }

    pub fn style(
        &self,
        effects: &[EffectRegion],
        fx: &ClickFxSettings,
    ) -> (SpotlightMode, f32, f32, f32) {
        let active_region = self.driver.map(|i| &effects[i]);
        (
            active_region
                .and_then(|e| e.mode)
                .unwrap_or(fx.spotlight_mode),
            active_region
                .and_then(|e| e.dim)
                .unwrap_or(fx.spotlight_dim),
            active_region
                .and_then(|e| e.radius)
                .unwrap_or(fx.spotlight_radius),
            active_region
                .and_then(|e| e.feather)
                .unwrap_or(fx.spotlight_feather),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(
        id: &str,
        start_ms: u32,
        end_ms: u32,
        fade_in_ms: u32,
        fade_out_ms: u32,
        mode: Option<SpotlightMode>,
        layer: u32,
    ) -> EffectRegion {
        EffectRegion {
            id: id.into(),
            kind: EffectKind::Spotlight,
            start_ms,
            end_ms,
            fade_in_ms,
            fade_out_ms,
            mode,
            dim: None,
            radius: None,
            feather: None,
            layer,
            rect: None,
            strength: None,
            roundness: None,
        }
    }

    #[test]
    fn spotlight_uses_per_region_fades() {
        let e = region("e0", 0, 1000, 100, 500, None, 0);
        let a = region_alpha(&e, 50);
        assert!((a - 0.5).abs() < 0.05, "fade-in half way: {a}");
        let b = region_alpha(&e, 750);
        assert!((b - 0.5).abs() < 0.05, "fade-out half way: {b}");
    }

    #[test]
    fn highest_layer_region_wins_style_not_first_match() {
        let a = region("e0", 0, 2000, 100, 100, Some(SpotlightMode::Classic), 0);
        let b = region("e1", 0, 2000, 100, 100, Some(SpotlightMode::Nebula), 1);
        let mut sim = SpotlightSim::new();
        sim.resolve(&[a.clone(), b.clone()], 0, false);
        let (mode, _, _, _) = sim.style(&[a, b], &ClickFxSettings::default());
        assert_eq!(
            mode,
            SpotlightMode::Nebula,
            "the higher-layer region (b) should win, not the first match (a)"
        );
    }

    #[test]
    fn spotlight_handoff_eases_alpha_instead_of_jump_maxing() {
        let a = region("e0", 0, 2000, 100, 100, None, 0);
        let b = region("e1", 500, 600, 300, 300, None, 1);
        let mut sim = SpotlightSim::new();
        let mut alpha = 0.0;
        for et in (0..500).step_by(16) {
            alpha = sim.resolve(&[a.clone(), b.clone()], et, false);
        }
        assert!(alpha > 0.9, "a should be fully faded in by t=500: {alpha}");
        let after = sim.resolve(&[a, b], 520, false);
        assert!(
            after > 0.8,
            "alpha should not jump-drop toward b's own barely-started fade: {after}"
        );
    }

    #[test]
    fn a_mask_region_never_drives_the_spotlight() {
        let mut blur = region("m0", 0, 10_000, 250, 250, None, 9);
        blur.kind = EffectKind::Blur;
        let effects = [blur, region("e0", 3000, 6000, 250, 250, None, 0)];
        let mut sim = SpotlightSim::new();
        assert_eq!(sim.resolve(&effects, 1000, false), 0.0);
        assert!(sim.resolve(&effects, 4500, false) > 0.0);
    }
}
