#!/usr/bin/env python3
"""Generate the NSIS/WiX installer bitmaps from the app icon.

Run from anywhere: `python make_bitmaps.py` (paths below are resolved
relative to this script). Produces sidebar.bmp, header.bmp, dialog.bmp and
banner.bmp next to this file. All outputs are 24-bit BMP (PIL mode "RGB"),
as required by both NSIS and WiX -- see README.md for the size/role table.
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
ICON_PATH = HERE / ".." / "icons" / "icon.png"
FONT_PATH = Path("C:/Windows/Fonts/segoeuib.ttf")

# Deep-navy gradient lifted from the app icon's own shadow layers
# (src-tauri/icons/source.svg strokes #0d1c52 / #132a80).
NAVY_TOP = (0x0D, 0x1C, 0x52)
NAVY_BOTTOM = (0x13, 0x2A, 0x80)
# Near-white the WiX dialogs draw their own black text over.
LIGHT = (0xF6, 0xF7, 0xFB)
WHITE = (0xFF, 0xFF, 0xFF)


def vertical_gradient(size, top, bottom):
    w, h = size
    column = Image.new("RGB", (1, h))
    for y in range(h):
        t = y / max(h - 1, 1)
        column.putpixel((0, y), (
            round(top[0] + (bottom[0] - top[0]) * t),
            round(top[1] + (bottom[1] - top[1]) * t),
            round(top[2] + (bottom[2] - top[2]) * t),
        ))
    return column.resize((w, h))


def paste_icon(bg, icon, size, center):
    resized = icon.resize((size, size), Image.LANCZOS)
    xy = (round(center[0] - size / 2), round(center[1] - size / 2))
    bg.paste(resized, xy, resized)


def draw_text(bg, text, font_size, xy, anchor, fill=WHITE):
    ImageDraw.Draw(bg).text(xy, text, font=ImageFont.truetype(str(FONT_PATH), font_size), fill=fill, anchor=anchor)


def make_sidebar(icon):
    """164x314 -- NSIS welcome/finish page, left panel."""
    w, h = 164, 314
    bg = vertical_gradient((w, h), NAVY_TOP, NAVY_BOTTOM)
    paste_icon(bg, icon, 96, (w / 2, 84))
    draw_text(bg, "TCursor", 23, (w / 2, 156), anchor="mm")
    return bg


def make_header(icon):
    """150x57 -- NSIS top-right strip on interior pages."""
    w, h = 150, 57
    bg = vertical_gradient((w, h), NAVY_TOP, NAVY_BOTTOM)
    icon_size = 40
    icon_cx = w - 10 - icon_size / 2
    paste_icon(bg, icon, icon_size, (icon_cx, h / 2))
    draw_text(bg, "TCursor", 14, (icon_cx - icon_size / 2 - 8, h / 2 + 1), anchor="rm")
    return bg


def make_dialog(icon):
    """493x312 -- WiX welcome/exit dialog background (text drawn by WiX)."""
    w, h = 493, 312
    bg = Image.new("RGB", (w, h), LIGHT)
    split = round(w * 0.6)
    hero_w = w - split
    bg.paste(vertical_gradient((hero_w, h), NAVY_TOP, NAVY_BOTTOM), (split, 0))
    paste_icon(bg, icon, 130, (split + hero_w / 2, h / 2))
    return bg


def make_banner(icon):
    """493x58 -- WiX top banner on interior dialogs (text drawn by WiX)."""
    w, h = 493, 58
    bg = Image.new("RGB", (w, h), LIGHT)
    split = round(w * 0.75)
    hero_w = w - split
    bg.paste(vertical_gradient((hero_w, h), NAVY_TOP, NAVY_BOTTOM), (split, 0))
    paste_icon(bg, icon, 40, (split + hero_w / 2, h / 2))
    return bg


def main():
    icon = Image.open(ICON_PATH).convert("RGBA")
    outputs = {
        "sidebar.bmp": make_sidebar(icon),
        "header.bmp": make_header(icon),
        "dialog.bmp": make_dialog(icon),
        "banner.bmp": make_banner(icon),
    }
    for name, img in outputs.items():
        assert img.mode == "RGB", f"{name} is {img.mode}, expected RGB"
        img.save(HERE / name, "BMP")
        print(f"wrote {name}: {img.size[0]}x{img.size[1]} {img.mode}")


if __name__ == "__main__":
    main()
