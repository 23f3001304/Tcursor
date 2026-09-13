import oai, pathlib, urllib.error, time
MODEL = "gpt-image-2.5-sunburst"
STYLE = (" Rendered like a flagship macOS default wallpaper: photoreal, ultra clean, wide cinematic composition, soft "
         "natural light, calm and spacious with an uncluttered middle where a window could sit, subtle depth haze, no people, "
         "no text, no logos, no vignette, no grain.")
SCENES = {
  "coast-dusk":   "A rugged Pacific coastline seen from high above at dusk: dark cliffs, a long curved beach, deep blue sea, the last warm light on the horizon.",
  "island-sea":   "A small green island in a deep turquoise sea seen from the air on a bright clear day, gentle white surf lines, vast calm water.",
  "dunes":        "Endless sand dunes at golden hour, long soft shadows, warm apricot and rose tones fading into a pale sky.",
  "peaks-night":  "Snow-capped granite peaks under a deep indigo night sky with a faint band of stars, cool blue moonlight on the snow.",
  "forest-mist":  "A redwood forest at dawn with soft sunbeams through low mist, deep greens and warm gold, seen from a low wide angle.",
  "canyon":       "Layered sandstone canyon walls in deep reds and oranges under a pale morning sky, smooth flowing rock shapes, minimal.",
  "lake-glass":   "A perfectly still alpine lake at blue hour mirroring soft mountains, glassy water, cool blues with a hint of pink in the sky.",
  "hills-flat":   "Rolling hills at sunset drawn as smooth flat-colour illustrated shapes with soft gradients, in the style of an abstract vector landscape: layered dusky purples, oranges and deep blues, a single small sun.",
  "waves-aerial": "Ocean waves seen straight down from high above, long white foam lines on deep teal water, abstract and rhythmic.",
  "aurora-sky":   "A quiet snowfield under a wide green and violet aurora, faint stars, minimal foreground, vast sky.",
}
d = pathlib.Path("scenic"); d.mkdir(exist_ok=True)
for name, p in SCENES.items():
    out = d / f"{name}.png"
    if out.exists(): continue
    for attempt in range(3):
        try: oai.gen(str(out.with_suffix("")), p + STYLE, "1536x1024", "medium", MODEL); break
        except urllib.error.HTTPError as e: print("  retry", name, e.code, e.read().decode()[:200]); time.sleep(6)
print("DONE scenic")
