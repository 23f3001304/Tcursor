// The one Win32 seam of the cursor layer: turn a live `HCURSOR` into its actual bitmap. Every
// pixel rule lives in `cursorpixels` (pure, tested); this file only talks to GDI and hands the
// bytes over. Windows-only - `events/track/mod.rs` gates the whole module.
use std::ffi::c_void;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, HCURSOR, HICON, ICONINFO};

use crate::events::track::cursorpixels::{color_rgba, mono_rgba, CapturedCursor};

/// The bitmap `cursor` is currently drawn from, or `None` when Windows will not describe it
/// (`GetIconInfo` fails for some protected/remote-session cursors) or the bitmaps are unreadable.
///
/// Animated cursors (`.ani`) are NOT animated here: `GetIconInfo` reports the frame showing at
/// this instant, and only the first one seen is kept - a spinning "busy" cursor therefore renders
/// as a still. Documented simplification; `DrawIconEx` (which can step frames) is not used.
pub fn capture(cursor: HCURSOR) -> Option<CapturedCursor> {
    unsafe {
        let mut ii = ICONINFO::default();
        GetIconInfo(HICON(cursor.0), &mut ii).ok()?;
        let out = with_dc(&ii);
        // Both handles are OURS the moment GetIconInfo succeeds - leaking them leaks a GDI object
        // per poll, so they are freed whatever `with_dc` returned.
        if !ii.hbmMask.0.is_null() {
            let _ = DeleteObject(HGDIOBJ::from(ii.hbmMask));
        }
        if !ii.hbmColor.0.is_null() {
            let _ = DeleteObject(HGDIOBJ::from(ii.hbmColor));
        }
        out
    }
}

/// Borrow the screen DC for the `GetDIBits` reads, and always give it back.
unsafe fn with_dc(ii: &ICONINFO) -> Option<CapturedCursor> {
    let hdc = GetDC(HWND(std::ptr::null_mut()));
    if hdc.0.is_null() {
        return None;
    }
    let out = read(ii, hdc);
    ReleaseDC(HWND(std::ptr::null_mut()), hdc);
    out
}

/// The two shapes a Win32 cursor comes in: a colour bitmap plus an AND mask, or a NULL colour
/// bitmap and a double-height mask holding the AND and XOR halves.
unsafe fn read(ii: &ICONINFO, hdc: HDC) -> Option<CapturedCursor> {
    let mask = describe(ii.hbmMask)?;
    let (mw, mh) = (mask.bmWidth.max(0) as u32, mask.bmHeight.max(0) as u32);
    let hot = (ii.xHotspot, ii.yHotspot);
    if ii.hbmColor.0.is_null() {
        let h = mh / 2;
        let bits = dib32(hdc, ii.hbmMask, mw, mh)?;
        let rgba = mono_rgba(&bits, mw, h, true)?;
        return Some(CapturedCursor { w: mw, h, hx: hot.0, hy: hot.1, rgba });
    }
    let color = describe(ii.hbmColor)?;
    let (w, h) = (color.bmWidth.max(0) as u32, color.bmHeight.max(0) as u32);
    let bgra = dib32(hdc, ii.hbmColor, w, h)?;
    // The mask is only consulted for a 32bpp cursor with no real alpha; an unreadable one is not
    // fatal (`color_rgba` then leaves the sprite opaque rather than fully transparent).
    let and = dib32(hdc, ii.hbmMask, w, h).unwrap_or_default();
    let rgba = color_rgba(&bgra, &and, w, h, true)?;
    Some(CapturedCursor { w, h, hx: hot.0, hy: hot.1, rgba })
}

/// The `BITMAP` header of a GDI bitmap (width/height/depth), or `None` if it has none.
unsafe fn describe(h: HBITMAP) -> Option<BITMAP> {
    let mut bm = BITMAP::default();
    let n = GetObjectW(
        HGDIOBJ::from(h),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut BITMAP as *mut c_void),
    );
    (n != 0 && bm.bmWidth > 0 && bm.bmHeight > 0).then_some(bm)
}

/// `rows` scanlines of `h`, converted to 32bpp BI_RGB. `biHeight` is NEGATIVE, which is what asks
/// GDI for top-down rows instead of a DIB's default bottom-up order (see `cursorpixels::row`).
unsafe fn dib32(hdc: HDC, h: HBITMAP, w: u32, rows: u32) -> Option<Vec<u8>> {
    if w == 0 || rows == 0 {
        return None;
    }
    let mut bi = BITMAPINFO::default();
    bi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: w as i32,
        biHeight: -(rows as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let mut buf = vec![0u8; w as usize * rows as usize * 4];
    let got = GetDIBits(
        hdc,
        h,
        0,
        rows,
        Some(buf.as_mut_ptr() as *mut c_void),
        &mut bi,
        DIB_RGB_COLORS,
    );
    (got != 0).then_some(buf)
}
