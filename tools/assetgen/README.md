# assetgen - how the bundled wallpapers and cursor packs were made

Small Python scripts (Pillow + the OpenAI Images API, no other deps) that produced
`src-tauri/assets/backgrounds/wallpapers/` and `src-tauri/assets/cursorpacks/` on 2026-09-13.
Kept so the sets are reproducible and extendable rather than a pile of opaque binaries.

- `oai.py` - the one API call (`gen`), model `gpt-image-2.5-sunburst`; reads `OPENAI_API_KEY` from the
  environment. Run from this directory.
- `batch.py cursors|wallpapers` - the "Clean" cursor pack (nine states, transparent PNG) and the twelve
  "Ribbons" wallpapers.  `folds.py` - the twelve sculptural "Folds" wallpapers.  `scenic.py` - the ten
  photoreal "Scenic" wallpapers.  `packs.py [theme ...]` - the themed cursor packs (one prompt per
  state, one style spec per theme; add a theme to `THEMES` to make a new pack).
- `mkpack.py <src> <id> "<Name>" <out>` - crops, pads and resizes generated cursor PNGs into a pack
  folder (`SIZE` px; the bundled packs use 128), derives hotspots (topmost opaque pixel for
  `arrow`/`hand`, bounding-box centre otherwise) and writes `hotspots.json` + `pack.json` + a
  contact sheet.  `mkbusy.py <id> spin|flip|pulse` - previews the busy animation as a GIF (the app
  synthesises these frames itself from the single `busy.png`; see the cursorpacks README).

Wallpapers are generated at 1536x1024 and centre-cropped to 16:9 (1536x864) as JPEG q90.
Everything generated this way is part of the repository under its MIT license.
