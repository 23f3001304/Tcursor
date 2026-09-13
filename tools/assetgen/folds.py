import oai, pathlib, urllib.error, time
MODEL = "gpt-image-2.5-sunburst"
STYLE = (" Style: a flagship macOS default wallpaper in the sculptural 'silk folds' language - large, smooth, glossy 3D folds "
         "and hills of fabric-like surface with soft studio lighting, deep shadows in the valleys and bright specular highlights on "
         "the crests, bold saturated colour blocking, ultra smooth gradients, no texture, no grain, no text, no logos, 16:9 wide.")
SETS = {
  "silk-blue":     "Deep royal blue and violet silk folds rising diagonally, a near-black navy shadow in the valleys, one bright cobalt highlight crest.",
  "sonoma-hills":  "Rolling hills of smooth lime and forest green in the foreground, with a wide band of coral pink and lavender blue folds sweeping across the sky above.",
  "sunset-folds":  "Folds of hot orange, magenta and soft peach rising like dunes, deep plum shadows, a cool violet fold at the top edge.",
  "ocean-folds":   "Teal and aquamarine silk folds like slow ocean swells, deep petrol shadows, a pale seafoam highlight.",
  "graphite-silk": "Monochrome graphite and charcoal silk folds with silver highlights - elegant, dark, no colour.",
  "lavender-mint": "Soft lavender and mint folds with a pale cream crest, gentle pastel lighting, airy and light.",
  "ember":         "Dark burgundy and ember red folds rising from a black valley, one gold-lit crest.",
  "aurora-folds":  "Emerald green and deep teal folds with a violet band along the top, like an aurora frozen into fabric.",
  "brand-folds":   "Royal blue (#2b53e6) silk folds over midnight navy (#0d1c52) shadows with one thin coral (#ef4444) fold crossing the upper right.",
  "peach-sky":     "Peach, rose and pale sky-blue folds like a soft dawn, low contrast, very smooth, bright.",
  "forest-night":  "Deep forest green folds under a dark indigo sky band, moonlit highlights on the crests.",
  "citrus":        "Bold lemon yellow and tangerine folds with a hot pink valley, energetic but smooth.",
}
d = pathlib.Path("folds"); d.mkdir(exist_ok=True)
for name, p in SETS.items():
    out = d / f"{name}.png"
    if out.exists(): continue
    for attempt in range(3):
        try: oai.gen(str(out.with_suffix("")), p + STYLE, "1536x1024", "medium", MODEL); break
        except urllib.error.HTTPError as e: print("  retry", name, e.code, e.read().decode()[:200]); time.sleep(6)
print("DONE folds")
