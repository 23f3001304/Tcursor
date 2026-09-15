use super::*;
use windows_capture::window::Window;

#[test]
fn test_enum_targets() {
    let targets = list_targets();
    println!("--- TARGETS ({}) ---", targets.len());
    for t in &targets {
        println!("  [{:?}] {} ({})", t.kind, t.label, t.id);
    }

    if let Ok(m) = Monitor::from_index(0) {
        println!(
            "Monitor 0 ok: {}x{}",
            m.width().unwrap_or(0),
            m.height().unwrap_or(0)
        );
    }

    if let Some(TargetId::Window(handle)) = targets
        .iter()
        .map(|t| t.id)
        .find(|id| matches!(id, TargetId::Window(_)))
    {
        let w = Window::from_raw_hwnd(handle as usize as *mut std::ffi::c_void);
        println!("Window from_raw_hwnd ok: title={:?}", w.title());
    }
}
