import sys, oai, pathlib, urllib.error, time
MODEL = "gpt-image-2.5-sunburst"
BASE = ("Render it centered, large, completely isolated on a transparent background. No text, nothing else in the image. "
        "One consistent design language for the whole pack.")
STATES = {
  "arrow":       "the standard ARROW pointer, tip pointing to the upper-left",
  "ibeam":       "the TEXT I-BEAM (a vertical bar with short serifs at the top and bottom)",
  "hand":        "the POINTING HAND (link cursor), index finger pointing up",
  "resize_ns":   "a VERTICAL double-headed resize arrow (up and down)",
  "resize_ew":   "a HORIZONTAL double-headed resize arrow (left and right)",
  "resize_nwse": "a DIAGONAL double-headed resize arrow from upper-left to lower-right",
  "resize_nesw": "a DIAGONAL double-headed resize arrow from upper-right to lower-left",
  "move":        "the MOVE cursor, four arrows pointing up, down, left and right from one center",
  "busy":        "a BUSY / loading indicator",
}
THEMES = {
  "cat": ("Cat-themed cursor pack, cute and clean, soft rounded shapes, cream-white fur with pink paw pads and a thin warm-brown outline, small soft shadow.", {
      "arrow": "a cat's front paw pointing to the upper-left like an arrow pointer, pink toe beans visible",
      "ibeam": "a cat's tail held straight up as a text I-beam, with a small tuft at the tip",
      "hand": "a cat paw raised and pointing up (the link cursor), pads facing the viewer",
      "resize_ns": "two cat paws back to back, one pointing up and one down, as a vertical resize arrow",
      "resize_ew": "two cat paws back to back, one pointing left and one right, as a horizontal resize arrow",
      "resize_nwse": "two cat paws back to back along the upper-left to lower-right diagonal, as a resize arrow",
      "resize_nesw": "two cat paws back to back along the upper-right to lower-left diagonal, as a resize arrow",
      "move": "a small round cat face in the center with four paws pointing up, down, left and right",
      "busy": "a sleeping curled-up cat forming a ring, one tiny 'z' floating above",
  }),
  "classic-2001": ("Early-2000s desktop OS cursor style: solid white fill, a crisp 1-pixel black outline, a hard offset drop shadow, slightly chunky proportions, nostalgic but clean.", {
      "busy": "a BUSY indicator drawn as a classic sand hourglass, white with a black outline",
      "hand": "the classic pointing hand cursor: a white glove-like hand with a black outline, index finger up",
  }),
  "aero-glass": ("Glassy translucent cursor style: a frosted pale-blue glass body with a glossy white highlight along the top edge, soft inner glow, thin darker blue rim, gentle shadow; premium and smooth.", {
      "busy": "a BUSY indicator drawn as a glossy blue ring with a brighter segment, like a spinning loading ring",
  }),
  "cartoon": ("Bold cartoon cursor style: thick black outlines, bouncy exaggerated shapes, flat bright colours (sunny yellow fill with an orange edge), a hard black drop shadow, playful comic look.", {
      "busy": "a BUSY indicator drawn as a cartoon spinning star burst ring",
  }),
  "glove": ("Vintage cartoon white glove cursor pack: a plump white four-fingered glove with three short black stitch lines on the back, thick black outline, rubber-hose animation style; every state is the glove in a pose.", {
      "arrow": "the glove with the index finger pointing to the upper-left as the main pointer",
      "ibeam": "the glove holding a thin vertical bar (a text I-beam) between finger and thumb",
      "hand": "the glove with the index finger pointing straight up (the link cursor)",
      "resize_ns": "the glove gripping a vertical double-headed arrow",
      "resize_ew": "the glove gripping a horizontal double-headed arrow",
      "resize_nwse": "the glove gripping a diagonal double-headed arrow running upper-left to lower-right",
      "resize_nesw": "the glove gripping a diagonal double-headed arrow running upper-right to lower-left",
      "move": "the glove open-palmed with four small arrows around it pointing up, down, left and right",
      "busy": "the glove holding a pocket watch as a busy indicator",
  }),
  "pixel": ("8-bit pixel-art cursor style: visibly chunky square pixels, a limited palette of white fill and pure black outline with one grey shading tone, no anti-aliasing, retro video-game look.", {
      "busy": "a BUSY indicator drawn as a pixel-art hourglass",
  }),
  "neon": ("Neon cyberpunk cursor style: a hollow shape drawn as a glowing magenta (#ff2d95) neon tube with a cyan (#3ff0ff) inner line, dark night-black core, soft neon glow.", {
      "busy": "a BUSY indicator drawn as a glowing neon ring with a gap",
  }),
  "paper": ("Paper cut-out craft cursor style: layered paper shapes in off-white with a warm cream second layer peeking underneath, subtle paper texture, soft realistic drop shadow, handmade feel.", {
      "busy": "a BUSY indicator drawn as a paper cut-out ring with a gap",
  }),
  "ink": ("Hand-drawn ink brush cursor style: a single confident black ink brush stroke shape with slightly rough edges and a little ink texture, monochrome, on nothing.", {
      "busy": "a BUSY indicator drawn as an enso-style brushed ink circle with a gap",
  }),
  "clay": ("Soft 3D clay cursor style: matte pastel clay shapes (lavender body with a mint accent), rounded puffy edges, soft studio lighting, gentle shadow, toy-like and friendly.", {
      "busy": "a BUSY indicator drawn as a puffy clay ring with a gap",
  }),
  # Crystal (2026-09-14): the owner's Liquid Glass cursor - NOT glyphs rendered in glass (Aero
  # Glass and Gradient Glass already are that) and NOT a clear 3D bubble (rejected on sight). It is
  # Apple's iOS 26 / macOS Tahoe Liquid Glass MATERIAL in the iPadOS pointer's shapes: a frosted,
  # flat-lensed disc for the pointer, the standard thin translucent text pill, oriented capsules
  # for the resizes, a dotted disc for move, a ring for busy. No arrow, no hand, no I-beam serifs.
  # Hotspots are the CENTRE of every sprite (mkpack's tip rule is overridden for this pack). The
  # renderer refracts the frame through the sprite at run time (pack.json `material: "glass"`).
  "crystal": ("Apple iOS 26 Liquid Glass material, seen straight on: a smooth, FLAT slab of frosted translucent glass with a "
      "soft milky-white fill at roughly 40 percent opacity, a thin crisp bright specular highlight tracing the upper-left "
      "rim and a fainter one along the lower-right rim, a subtle soft inner shadow just inside the edge that gives it "
      "depth, gentle lens-like brightening at the edges, a faint cool neutral tint; calm, minimal, premium, exactly like "
      "the pointer and text cursor of iPadOS; NOT a 3D marble, NOT a soap bubble, NOT chrome; NO drop shadow, NO "
      "background, NO arrow, NO hand.", {
      "arrow": "the pointer as a perfectly round frosted-glass disc, the iPadOS pointer circle, with no arrow shape at all",
      "ibeam": "the text cursor as a HORIZONTAL rounded pill of frosted glass, about three times wider than tall, like a highlight capsule lying over a line of text, no serifs",
      "hand": "the link cursor as a slightly larger round frosted-glass disc with a faint brighter ring just inside its edge",
      "resize_ns": "a VERTICAL rounded capsule of frosted glass, long and slim, meaning up-and-down resizing",
      "resize_ew": "a HORIZONTAL rounded capsule of frosted glass, long and slim, meaning left-and-right resizing",
      "resize_nwse": "a slim rounded capsule of frosted glass lying along the upper-left to lower-right diagonal",
      "resize_nesw": "a slim rounded capsule of frosted glass lying along the upper-right to lower-left diagonal",
      "move": "a round frosted-glass disc with four tiny frosted-glass dots floating just outside it at top, bottom, left and right",
      "busy": "a BUSY indicator drawn as a frosted-glass ring with one brighter segment",
  }),
}
which = sys.argv[1:] or list(THEMES)
for theme in which:
    style, over = THEMES[theme]
    d = pathlib.Path("packs") / theme; d.mkdir(parents=True, exist_ok=True)
    for kind, base in STATES.items():
        out = d / f"{kind}.png"
        if out.exists(): continue
        desc = over.get(kind, base)
        prompt = f"A single mouse cursor for a themed cursor pack: {desc}. {style} {BASE}"
        for attempt in range(3):
            try:
                oai.gen(str(out.with_suffix("")), prompt, "1024x1024", "medium", MODEL, transparent=True); break
            except urllib.error.HTTPError as e:
                print("  retry", theme, kind, e.code, e.read().decode()[:200]); time.sleep(6)
    print("DONE theme", theme)
print("ALL DONE")
