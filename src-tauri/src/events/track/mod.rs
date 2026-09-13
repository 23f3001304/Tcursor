pub mod tracker;
pub mod cursortracker;
pub mod cursortype;
/// The Win32 seam that reads a live `HCURSOR`'s bitmap - Windows-only, like the tracker's `imp`.
#[cfg(windows)]
pub mod cursorcapture;
pub mod cursorlayer;
pub mod cursorpixels;
pub mod typing;
