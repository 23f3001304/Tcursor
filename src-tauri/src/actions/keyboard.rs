use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use crate::actions::matcher::{Arm, Mods};
use crate::actions::model::ActionEvent;

/// Printable typing keys (digits, letters, space, enter, backspace, tab). We stamp a
/// timestamp on each fresh DOWN of any of these - NEVER which key (privacy).
const TYPING_VKS: &[u32] = &[
    0x30,0x31,0x32,0x33,0x34,0x35,0x36,0x37,0x38,0x39,
    0x41,0x42,0x43,0x44,0x45,0x46,0x47,0x48,0x49,0x4A,0x4B,0x4C,0x4D,
    0x4E,0x4F,0x50,0x51,0x52,0x53,0x54,0x55,0x56,0x57,0x58,0x59,0x5A,
    0x20,0x0D,0x08,0x09,
];

#[cfg(windows)]
fn key_down(vk: u32) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}
#[cfg(not(windows))]
fn key_down(_vk: u32) -> bool { false }

/// Live modifier state from the global async key state (VK_CONTROL/MENU/SHIFT).
fn cur_mods() -> Mods {
    Mods { ctrl: key_down(0x11), alt: key_down(0x12), shift: key_down(0x10) }
}

/// Captures hotkey actions by POLLING the async key state (~60Hz), not a
/// WH_KEYBOARD_LL hook: anti-keylogger/security layers silently swallow that hook on
/// some machines (it installs but never fires) while the mouse hook keeps working.
/// Polling reads physical state directly and inspects ONLY the configured chord keys
/// (modifiers + each armed main key) -- it is not a general keylogger.
pub struct KeyboardTracker {
    stop: Arc<AtomicBool>,
    events: Arc<Mutex<Vec<ActionEvent>>>,
    typing: Arc<Mutex<Vec<u32>>>,
    thread: Option<JoinHandle<()>>,
}

impl KeyboardTracker {
    pub fn start(arms: Vec<Arm>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let events: Arc<Mutex<Vec<ActionEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let typing: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
        let (s, e, ty) = (stop.clone(), events.clone(), typing.clone());
        let thread = std::thread::Builder::new()
            .name("keyboard-poll".into())
            .spawn(move || {
                let start = Instant::now();
                let mut active = vec![false; arms.len()];
                let mut typ_prev = vec![false; TYPING_VKS.len()];
                while !s.load(Ordering::SeqCst) {
                    let mods = cur_mods();
                    for (i, arm) in arms.iter().enumerate() {
                        let now = key_down(arm.chord.vk) && mods == arm.chord.mods;
                        if now == active[i] { continue; }
                        active[i] = now;
                        let kind = if now { Some(arm.on_down) } else { arm.on_up };
                        if let Some(kind) = kind {
                            let t = start.elapsed().as_millis() as u32;
                            if let Ok(mut g) = e.lock() { g.push(ActionEvent { t, kind }); }
                        }
                    }
                    for (k, &vk) in TYPING_VKS.iter().enumerate() {
                        let d = key_down(vk);
                        if d && !typ_prev[k] {
                            let t = start.elapsed().as_millis() as u32;
                            if let Ok(mut g) = ty.lock() { g.push(t); }
                        }
                        typ_prev[k] = d;
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
            })
            .ok();
        Self { stop, events, typing, thread }
    }

    pub fn stop(mut self) -> (Vec<ActionEvent>, Vec<u32>) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() { let _ = t.join(); }
        let actions = std::mem::take(&mut *self.events.lock().unwrap_or_else(|e| e.into_inner()));
        let typing = std::mem::take(&mut *self.typing.lock().unwrap_or_else(|e| e.into_inner()));
        (actions, typing)
    }
}

impl Drop for KeyboardTracker {
    // Stop the poll thread if dropped without stop() (e.g. a start_recording error path).
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.stop.store(true, Ordering::SeqCst);
            if let Some(t) = self.thread.take() { let _ = t.join(); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_returns_tuple_with_empty_vecs_when_no_input() {
        let tracker = KeyboardTracker::start(vec![]);
        let (actions, typing) = tracker.stop();
        assert!(actions.is_empty());
        assert!(typing.is_empty());
    }
}
