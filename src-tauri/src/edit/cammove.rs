use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CameraMove {
    pub id: String,
    pub t_ms: u32,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    #[serde(default = "default_cam_easing")]
    pub easing: String,
    #[serde(default = "default_cam_shape")]
    pub shape: String,
    #[serde(default = "default_cam_roundness")]
    pub roundness: f32,
}

pub const DEFAULT_CAM_ROUNDNESS: f32 = 0.12;
fn default_cam_easing() -> String {
    "smooth".into()
}
fn default_cam_shape() -> String {
    "layout".into()
}
fn default_cam_roundness() -> f32 {
    DEFAULT_CAM_ROUNDNESS
}
