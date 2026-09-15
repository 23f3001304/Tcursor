use crate::edit::model::{Arrangement, EditDoc, PanelPose};
use crate::edit::ops::api::EditOp;
use serde::{Deserialize, Deserializer};

const MIN_SIZE: f32 = 0.05;
const MAX_SIZE: f32 = 1.5;

pub(crate) fn clamp_pose(p: PanelPose) -> PanelPose {
    PanelPose {
        cx: p.cx.clamp(0.0, 1.0),
        cy: p.cy.clamp(0.0, 1.0),
        size: p.size.clamp(MIN_SIZE, MAX_SIZE),
    }
}

pub fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(d).map(Some)
}

pub fn apply_arrangement(doc: &mut EditDoc, op: EditOp) {
    match op {
        EditOp::SetArrangement { id, screen, cam } => {
            if let Some(s) = doc.layout.iter_mut().find(|s| s.id == id) {
                let cur = s.arrangement.unwrap_or(Arrangement {
                    screen: None,
                    cam: None,
                });
                let pick =
                    |touched: Option<Option<PanelPose>>, now: Option<PanelPose>| match touched {
                        None => now,
                        Some(v) => v.map(clamp_pose),
                    };
                let next = Arrangement {
                    screen: pick(screen, cur.screen),
                    cam: pick(cam, cur.cam),
                };
                if next.screen.is_some() || next.cam.is_some() {
                    s.arrangement = Some(next);
                }
            }
        }
        EditOp::ClearArrangement { id } => {
            if let Some(s) = doc.layout.iter_mut().find(|s| s.id == id) {
                s.arrangement = None;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "arrangement_tests.rs"]
mod tests;
