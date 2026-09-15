use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    System,
    Enhanced,
    Hidden,
}
impl CursorStyle {
    pub fn captures_os_cursor(self) -> bool {
        matches!(self, CursorStyle::System)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CursorBack {
    #[default]
    None,
    Glass,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct CursorSettings {
    pub style: CursorStyle,
    pub size: f32,
    pub smoothness: f32,
    pub path_idealize: f32,
    pub motion_blur: f32,
    #[serde(default = "default_tilt")]
    pub tilt: f32,
    pub click_bounce: bool,
    pub bounce_intensity: f32,
    pub pack: String,
    pub back: CursorBack,
}

fn default_tilt() -> f32 {
    0.35
}
impl Default for CursorSettings {
    fn default() -> Self {
        Self {
            style: CursorStyle::System,
            size: 1.0,
            smoothness: 0.6,
            path_idealize: 0.0,
            motion_blur: 0.35,
            tilt: default_tilt(),
            click_bounce: true,
            bounce_intensity: 0.5,
            pack: "default".to_string(),
            back: CursorBack::None,
        }
    }
}
impl CursorSettings {
    pub fn plain_os(&self, os_cursor_in_video: bool) -> bool {
        self.style == CursorStyle::System && !os_cursor_in_video
    }
    pub fn smoothness_at(&self, os_cursor_in_video: bool) -> f32 {
        if self.plain_os(os_cursor_in_video) {
            0.0
        } else {
            self.smoothness.clamp(0.0, 1.0)
        }
    }
    pub fn idealize_at(&self, os_cursor_in_video: bool) -> f32 {
        if self.plain_os(os_cursor_in_video) {
            0.0
        } else {
            self.path_idealize
        }
    }
    pub fn tilt_at(&self, os_cursor_in_video: bool) -> f32 {
        if self.plain_os(os_cursor_in_video) {
            0.0
        } else {
            self.tilt
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_back_is_off_by_default_and_round_trips() {
        assert_eq!(CursorSettings::default().back, CursorBack::None);
        let json = serde_json::to_string(&CursorBack::Glass).unwrap();
        assert_eq!(
            json, "\"glass\"",
            "the wire name the TS `CursorBackStyle` mirrors"
        );
        assert_eq!(
            serde_json::from_str::<CursorBack>("\"none\"").unwrap(),
            CursorBack::None
        );
    }

    #[test]
    fn the_motion_tilt_round_trips_and_defaults_present_but_subtle() {
        assert_eq!(CursorSettings::default().tilt, 0.35);
        let json = serde_json::to_string(&CursorSettings {
            tilt: 0.8,
            ..Default::default()
        })
        .unwrap();
        assert!(
            json.contains("\"tilt\":0.8"),
            "the wire name the TS mirror reads: {json}"
        );
        assert_eq!(
            serde_json::from_str::<CursorSettings>(&json).unwrap().tilt,
            0.8
        );
        let old = r#"{"style":"enhanced","size":1.0,"smoothness":0.2,"pack":"crystal"}"#;
        let s: CursorSettings = serde_json::from_str(old).unwrap();
        assert_eq!((s.tilt, s.smoothness), (0.35, 0.2));
        let sys = CursorSettings {
            style: CursorStyle::System,
            tilt: 1.0,
            ..Default::default()
        };
        assert_eq!(sys.tilt_at(false), 0.0);
        assert_eq!(sys.tilt_at(true), 1.0);
    }

    #[test]
    fn a_settings_file_written_before_the_back_existed_still_loads() {
        let old = r#"{"style":"enhanced","size":1.0,"pack":"crystal"}"#;
        let s: CursorSettings = serde_json::from_str(old).unwrap();
        assert_eq!(s.back, CursorBack::None);
        assert_eq!(s.pack, "crystal");
    }
}
