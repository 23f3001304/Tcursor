use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use crate::actions::matcher::{Arm, Mods};
use crate::actions::model::ActionEvent;

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
    thread: Option<JoinHandle<()>>,
}

impl KeyboardTracker {
    pub fn start(arms: Vec<Arm>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let events: Arc<Mutex<Vec<ActionEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let (s, e) = (stop.clone(), events.clone());
        let thread = std::thread::Builder::new()
            .name("keyboard-poll".into())
            .spawn(move || {
                let start = Instant::now();
                let mut active = vec![false; arms.len()];
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
                    std::thread::sleep(Duration::from_millis(15));
                }
            })
            .ok();
        Self { stop, events, thread }
    }

    pub fn stop(mut self) -> Vec<ActionEvent> {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() { let _ = t.join(); }
        std::mem::take(&mut *self.events.lock().unwrap_or_else(|e| e.into_inner()))
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
