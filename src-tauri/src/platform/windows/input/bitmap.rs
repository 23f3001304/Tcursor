use std::ffi::c_void;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, HCURSOR, HICON, ICONINFO};

use crate::events::track::cursorpixels::{color_rgba, mono_rgba, CapturedCursor};

pub fn capture(cursor: HCURSOR) -> Option<CapturedCursor> {
    unsafe {
        let mut icon_info = ICONINFO::default();
        GetIconInfo(HICON(cursor.0), &mut icon_info).ok()?;
        let out = with_dc(&icon_info);
        if !icon_info.hbmMask.0.is_null() {
            let _ = DeleteObject(HGDIOBJ::from(icon_info.hbmMask));
        }
        if !icon_info.hbmColor.0.is_null() {
            let _ = DeleteObject(HGDIOBJ::from(icon_info.hbmColor));
        }
        out
    }
}

unsafe fn with_dc(icon_info: &ICONINFO) -> Option<CapturedCursor> {
    let hdc = GetDC(HWND(std::ptr::null_mut()));
    if hdc.0.is_null() {
        return None;
    }
    let out = read(icon_info, hdc);
    ReleaseDC(HWND(std::ptr::null_mut()), hdc);
    out
}

unsafe fn read(icon_info: &ICONINFO, hdc: HDC) -> Option<CapturedCursor> {
    let mask = describe(icon_info.hbmMask)?;
    let (mw, mh) = (mask.bmWidth.max(0) as u32, mask.bmHeight.max(0) as u32);
    let hot = (icon_info.xHotspot, icon_info.yHotspot);
    if icon_info.hbmColor.0.is_null() {
        let h = mh / 2;
        let bits = dib32(hdc, icon_info.hbmMask, mw, mh)?;
        let rgba = mono_rgba(&bits, mw, h, true)?;
        return Some(CapturedCursor {
            w: mw,
            h,
            hx: hot.0,
            hy: hot.1,
            rgba,
        });
    }
    let color = describe(icon_info.hbmColor)?;
    let (w, h) = (color.bmWidth.max(0) as u32, color.bmHeight.max(0) as u32);
    let bgra = dib32(hdc, icon_info.hbmColor, w, h)?;
    let and = dib32(hdc, icon_info.hbmMask, w, h).unwrap_or_default();
    let rgba = color_rgba(&bgra, &and, w, h, true)?;
    Some(CapturedCursor {
        w,
        h,
        hx: hot.0,
        hy: hot.1,
        rgba,
    })
}

unsafe fn describe(h: HBITMAP) -> Option<BITMAP> {
    let mut bm = BITMAP::default();
    let n = GetObjectW(
        HGDIOBJ::from(h),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut BITMAP as *mut c_void),
    );
    (n != 0 && bm.bmWidth > 0 && bm.bmHeight > 0).then_some(bm)
}

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
