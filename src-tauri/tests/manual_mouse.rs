use cursor_zoom_lib::platform::windows::input::pointer::Win32Pointer;
use cursor_zoom_lib::session::record::pause_totals::PauseTotals;
use std::sync::Arc;

#[test]
#[ignore]
fn logs_two_seconds_of_mouse() {
    let t = Win32Pointer::start(8, Arc::new(PauseTotals::new()));
    println!("move the mouse and click for 2s...");
    std::thread::sleep(std::time::Duration::from_secs(2));
    let events = t.stop();
    println!("captured {} events", events.len());
    assert!(events.len() > 1, "should capture mouse activity");
}

#[test]
#[ignore]
fn hook_captures_injected_moves() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT,
    };

    let tracker = Win32Pointer::start(0, Arc::new(PauseTotals::new()));
    std::thread::sleep(std::time::Duration::from_millis(50));

    for i in 1i32..=5 {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                mi: MOUSEINPUT {
                    dx: i * 3,
                    dy: i * 3,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    }

    std::thread::sleep(std::time::Duration::from_millis(50));
    let events = tracker.stop();
    println!(
        "hook_captures_injected_moves: captured {} events",
        events.len()
    );
    assert!(
        events.len() > 1,
        "hook must capture injected mouse moves; got {}",
        events.len()
    );
}
