use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::settings::model::HotkeySettings;

/// Live modifier state at a key event. Compared for exact equality against a chord.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Mods { pub ctrl: bool, pub alt: bool, pub shift: bool }

/// A parsed hotkey: modifier set + a single main virtual-key code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyChord { pub mods: Mods, pub vk: u32 }

impl KeyChord {
    /// Parse "Ctrl+Alt+Z" / "ctrl+alt+1" (case-insensitive, '+'-separated).
    /// Returns None unless exactly one main key (A-Z or 0-9) is present and every
    /// token is recognized.
    pub fn parse(s: &str) -> Option<KeyChord> {
        let mut mods = Mods::default();
        let mut vk: Option<u32> = None;
        for raw in s.split('+') {
            let tok = raw.trim();
            if tok.is_empty() { return None; }
            match tok.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => mods.ctrl = true,
                "alt" => mods.alt = true,
                "shift" => mods.shift = true,
                _ => {
                    let v = main_key_vk(tok)?;
                    if vk.is_some() { return None; } // a second main key
                    vk = Some(v);
                }
            }
        }
        Some(KeyChord { mods, vk: vk? })
    }

    pub fn matches(&self, vk: u32, mods: Mods) -> bool {
        self.vk == vk && self.mods == mods
    }
}

/// VK code for a single-character A-Z (0x41-0x5A) or 0-9 (0x30-0x39) token.
/// For these ranges the Win32 virtual-key equals the ASCII uppercase byte.
fn main_key_vk(tok: &str) -> Option<u32> {
    let mut chars = tok.chars();
    let c = chars.next()?;
    if chars.next().is_some() { return None; } // exactly one char
    let u = c.to_ascii_uppercase();
    if u.is_ascii_uppercase() || u.is_ascii_digit() { Some(u as u32) } else { None }
}

/// One armed chord and the action(s) it emits on press / release.
pub struct Arm { pub chord: KeyChord, pub on_down: ActionKind, pub on_up: Option<ActionKind> }

/// Matches live key events against the armed chords. Holds a "currently held" set
/// (arm indices) to suppress auto-repeat on press and to pair a release with its
/// press by main key even after the modifiers were let go first.
pub struct ActionMatcher { arms: Vec<Arm>, held: Vec<usize> }

impl ActionMatcher {
    pub fn new(arms: Vec<Arm>) -> Self { Self { arms, held: Vec::new() } }

    pub fn on_key(&mut self, down: bool, vk: u32, mods: Mods, t: u32) -> Option<ActionEvent> {
        if down {
            let i = self.arms.iter().position(|a| a.chord.matches(vk, mods))?;
            if self.held.contains(&i) { return None; } // auto-repeat
            self.held.push(i);
            Some(ActionEvent { t, kind: self.arms[i].on_down })
        } else {
            let pos = self.held.iter().position(|&i| self.arms[i].chord.vk == vk)?;
            let i = self.held.remove(pos);
            self.arms[i].on_up.map(|kind| ActionEvent { t, kind })
        }
    }
}

fn push_arm(arms: &mut Vec<Arm>, s: &str, on_down: ActionKind, on_up: Option<ActionKind>) {
    if let Some(chord) = KeyChord::parse(s) { arms.push(Arm { chord, on_down, on_up }); }
}

/// Build the armed-chord table from the configured hotkeys. Unparseable chords are
/// skipped (the recording still runs; that action is simply unbound).
pub fn arming_from_settings(h: &HotkeySettings) -> Vec<Arm> {
    let mut arms = Vec::new();
    push_arm(&mut arms, &h.zoom_hold, ActionKind::ZoomHoldStart, Some(ActionKind::ZoomHoldEnd));
    push_arm(&mut arms, &h.layout_screen, ActionKind::SetLayout(LayoutId::Screen), None);
    push_arm(&mut arms, &h.layout_camera, ActionKind::SetLayout(LayoutId::Camera), None);
    push_arm(&mut arms, &h.layout_presenter, ActionKind::SetLayout(LayoutId::Presenter), None);
    push_arm(&mut arms, &h.layout_screen_only, ActionKind::SetLayout(LayoutId::ScreenOnly), None);
    push_arm(&mut arms, &h.layout_camera_only, ActionKind::SetLayout(LayoutId::CameraOnly), None);
    arms
}

#[cfg(test)]
mod tests {
    use crate::actions::model::{ActionKind, LayoutId};
    use crate::settings::model::HotkeySettings;
    use super::*;

    fn ca() -> Mods { Mods { ctrl: true, alt: true, shift: false } }

    #[test]
    fn parses_letter_and_digit_case_insensitive() {
        let z = KeyChord::parse("Ctrl+Alt+Z").unwrap();
        assert_eq!(z.mods, ca());
        assert_eq!(z.vk, 'Z' as u32);
        let one = KeyChord::parse("ctrl+ALT+1").unwrap();
        assert!(one.mods.ctrl && one.mods.alt && !one.mods.shift);
        assert_eq!(one.vk, '1' as u32);
    }

    #[test]
    fn rejects_chords_without_a_single_main_key() {
        assert!(KeyChord::parse("Ctrl+Alt").is_none()); // modifiers only
        assert!(KeyChord::parse("Ctrl+Foo").is_none()); // unknown token
        assert!(KeyChord::parse("Ctrl+A+B").is_none()); // two main keys
        assert!(KeyChord::parse("").is_none());
    }

    #[test]
    fn matches_requires_exact_modifiers() {
        let z = KeyChord::parse("Ctrl+Alt+Z").unwrap();
        assert!(z.matches('Z' as u32, ca()));
        assert!(!z.matches('Z' as u32, Mods { ctrl: true, alt: false, shift: false }));
        assert!(!z.matches('X' as u32, ca()));
    }

    #[test]
    fn layout_emits_once_and_suppresses_autorepeat() {
        let mut m = ActionMatcher::new(arming_from_settings(&HotkeySettings::default()));
        let vk1 = '1' as u32; // default layout_screen = Ctrl+Alt+1
        let e = m.on_key(true, vk1, ca(), 10).unwrap();
        assert_eq!(e.kind, ActionKind::SetLayout(LayoutId::Screen));
        assert!(m.on_key(true, vk1, ca(), 20).is_none()); // auto-repeat down: suppressed
        assert!(m.on_key(false, vk1, ca(), 30).is_none()); // layout has no on_up
    }

    #[test]
    fn zoom_hold_pairs_start_then_end_even_if_mods_released_first() {
        let mut m = ActionMatcher::new(arming_from_settings(&HotkeySettings::default()));
        let vkz = 'Z' as u32; // default zoom_hold = Ctrl+Alt+Z
        assert_eq!(m.on_key(true, vkz, ca(), 0).unwrap().kind, ActionKind::ZoomHoldStart);
        // release Z after the user already let go of Ctrl/Alt — still pairs by vk
        let end = m.on_key(false, vkz, Mods::default(), 500).unwrap();
        assert_eq!(end.kind, ActionKind::ZoomHoldEnd);
        // a second release with nothing held is a no-op
        assert!(m.on_key(false, vkz, Mods::default(), 600).is_none());
    }

    #[test]
    fn unarmed_key_is_ignored() {
        let mut m = ActionMatcher::new(arming_from_settings(&HotkeySettings::default()));
        assert!(m.on_key(true, 'Q' as u32, ca(), 0).is_none());
    }
}
