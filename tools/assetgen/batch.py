import sys, oai, pathlib, urllib.error, time
MODEL = "gpt-image-2.5-sunburst"
STYLE = ("Style: modern, minimal macOS-like cursor - pure white fill, a thin dark navy (#0d1c52) outline, and a faint soft "
         "shadow offset slightly down-right. No glow, no bloom, no gradient, slightly rounded corners, flat clean vector look. "
         "Centered, large, completely isolated on a transparent background. No text, nothing else in the image.")
CURSORS = {
  "arrow":       "A single mouse cursor: the standard ARROW pointer, tip pointing to the upper-left.",
  "ibeam":       "A single mouse cursor: the TEXT I-BEAM (a vertical bar with short serifs at the top and bottom).",
  "hand":        "A single mouse cursor: the POINTING HAND (link cursor), index finger pointing up, seen from the back of the hand.",
  "resize_ns":   "A single mouse cursor: a VERTICAL double-headed resize arrow (arrowheads pointing up and down).",
  "resize_ew":   "A single mouse cursor: a HORIZONTAL double-headed resize arrow (arrowheads pointing left and right).",
  "resize_nwse": "A single mouse cursor: a DIAGONAL double-headed resize arrow running from the upper-left to the lower-right.",
  "resize_nesw": "A single mouse cursor: a DIAGONAL double-headed resize arrow running from the upper-right to the lower-left.",
  "move":        "A single mouse cursor: the MOVE cursor, four arrows pointing up, down, left and right from a common center.",
  "busy":        "A single mouse cursor: a BUSY / loading indicator, a circular ring with a short gap, drawn as a single clean stroke.",
}
WALL_STYLE = ("Abstract desktop wallpaper in the style of the macOS Sequoia and Sonoma wallpapers: a few large, layered, translucent "
              "glass-like ribbons flowing diagonally with soft volumetric light and gentle depth of field. Silky, ultra smooth, premium, "
              "calm; the center third stays quieter than the edges. No text, no logos, no grain. Palette: ")
WALLS = {
  "aurora":   "deep teal (#0b3a3f) through emerald (#1f8a70) with one pale cyan ribbon as the only highlight.",
  "sunset":   "deep plum (#2a1038) through magenta (#b03a6e) into a single warm amber (#f0a04b) ribbon.",
  "ocean":    "deep sea blue (#071f3a) through azure (#1b6fd6) with one bright cyan (#5ee6ff) edge highlight.",
  "graphite": "monochrome charcoal (#141416) through slate grey (#3a3d44) with one faint silver ribbon; no colour.",
  "sand":     "light: warm ivory (#f3ece2) through soft beige (#dcc7ab) with one terracotta (#c8703f) ribbon; bright and airy.",
  "sage":     "muted sage green (#5f7a63) through pale cream (#e9ede3) with one deep forest (#25402b) ribbon.",
  "plum":     "deep violet (#22103f) through lavender (#8f7bd8) with one soft pink (#f1a8c4) highlight.",
  "midnight": "near-black navy (#05091a) with a single thin electric blue (#2b53e6) ribbon of light and a faint deep-navy second ribbon.",
  "paper":    "light and subtle: off-white (#f7f6f2) through pale warm grey (#e2dfd8), very low contrast, one barely-there blue ribbon.",
  "rose":     "dusty rose (#a8636e) through blush cream (#f3dcd6) with one deep wine (#5a1f2b) ribbon.",
  "amber":    "dark: burnt orange (#7a2e0f) through amber (#e8a33a) on a near-black (#120a05) ground, one gold highlight.",
  "brand":    "deep midnight navy (#0d1c52) through royal blue (#2b53e6) with one warm coral (#ef4444) ribbon as the only accent.",
}
kind = sys.argv[1]
items = CURSORS if kind == "cursors" else WALLS
for name, p in items.items():
    out = pathlib.Path(kind) / f"{name}.png"
    if out.exists(): print("skip", out); continue
    prompt = (p + " " + STYLE) if kind == "cursors" else (WALL_STYLE + p)
    for attempt in range(3):
        try:
            oai.gen(str(out.with_suffix("")), prompt, "1024x1024" if kind == "cursors" else "1536x1024",
                    "medium" if kind == "cursors" else "high", MODEL, transparent=(kind == "cursors"))
            break
        except urllib.error.HTTPError as e:
            print("  retry", name, e.code, e.read().decode()[:200]); time.sleep(5)
print("DONE", kind)
