# Bundled wallpapers

Abstract 1536x864 JPEG (q90) wallpapers, generated for TCursor and part of this repository
under its MIT license. `bg.jpg` one level up is the legacy "Classic" mesh every pre-existing
project renders with; it deliberately stays out of this folder.

Provenance, all five sets: generated on 2026-09-13 with OpenAI `gpt-image-2.5-sunburst` via the
Images API from the project's own prompts (`tools/assetgen/`: `batch.py` for Ribbons, `folds.py`
for Folds, `gradients.py` for Gradients, `metal.py` for Metal, `scenic.py` for Scenic), at
1536x1024 and centre-cropped to 16:9. No sourced photographs or third-party images anywhere in
this folder. Record the same for any set added later.

## Adding one

Drop the JPEG in. `src-tauri/build.rs` scans this folder at build time and generates the table
`settings::wallpapers::WALLPAPERS` re-exports, so nothing is hand-listed anywhere. Everything
comes from the filename:

- **id** = the file stem. This is what `BackgroundSettings.mesh` persists, so it must stay
  stable - renaming a file changes what already-saved projects point at (they fall back to
  Classic).
- **group** = the prefix: `folds-` is Folds, `gradient-` is Gradients, `metal-` is Metal,
  `scenic-` is Scenic. No prefix means Ribbons, the original set, which predates the prefixes
  and keeps its bare ids.
- **name** = the stem minus that prefix, hyphens to spaces, title-cased
  (`scenic-coast-dusk` shows as "Coast Dusk").
- **order** in the picker = Ribbons, Folds, Gradients, Metal, Scenic; alphabetical by name
  inside each group.

They are embedded in the binary with `include_bytes!`, so keep them small: every file here is
compiled in whether or not anyone picks it.
