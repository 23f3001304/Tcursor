# src/editor/panels/BackgroundPanel.tsx

The Background rail panel: frame padding/corner radius, accent color, and a real background type/wallpaper/color/gradient/blur, all backed by `doc.settings.background` (`settings::background::BackgroundSettings` in Rust). Before these controls were wired, the type selector, presets, blur, frame-shadow, and remove-background controls were entirely disconnected from the renderer (a fixed background image/gradient was hardcoded in `FrameRenderer::new`, unconditionally, regardless of anything in this panel).

The two tile-heavy tabs live in their own files for the 200-line budget: `WallpaperGrid.tsx` (the shared tile row) and `GradientTab.tsx` (the gradient presets plus the custom stops and angle).

## BackgroundPanel

```tsx
export function BackgroundPanel({ folder, doc, onSaveSettings, onClose }: {
  folder: string; doc: EditDoc;
  onSaveSettings: (nextSettings: EditDoc["settings"]) => void; onClose: () => void;
}): JSX.Element
```

### Behavior

**Flow.** Five groups: **Background Type** and the chosen kind's own choices (what the background IS), then **Look** (Background Blur, Dim), then **Frame** (Corner Radius, Padding), then **Accent Colors** last. Every control and every name is unchanged; Accent Colors moved from the middle of the panel to the end because it is the editor's own accent and not part of the background at all, and the four unlabelled sliders that used to trail off the bottom now sit under the two headings that say what they do.

### Height (usability pass, 2026-09-13)

The owner's verdict on the panel pass was that Background "needs a lot of scrolling", and it did: 52 wallpapers three-up in five wrapping grids, plus Look, Frame and Accent, came to about 1939px in a slot that is roughly 620px tall on a 720px window.

Measured by rows at 320px wide, the Wallpapers tab is now:

| row | px |
|---|---|
| panel padding, top and bottom | 36 |
| header (title, lede, 16px gap) | 53 |
| Background Type (heading, gap, `Segmented`) | 57 |
| section gap | 16 |
| six rows (five groups plus Custom) at 55, five 8px gaps | 370 |
| section gap | 16 |
| the More row | 28 |
| **total** | **576** |

A row is an 11px label, a 4px gap, a 36px tile and a 4px reserved scrollbar lane. The other two tabs are smaller: Gradient 458, Color 364.

Three things bought back the 1363px:

1. **One `TileRow` per group** instead of a wrapping grid, about 700px (`TileRow.md` has the arithmetic).
2. **Two-up rows** (`.e-two`) for Blur | Dim and Corner Radius | Padding, 96px.
3. **The panel's one `Disclosure`**, holding Look, Frame and Accent Colors, 215px. Nothing is removed and nothing is renamed; those three sections are tuning applied to a background you have already chosen, so choosing stays in sight and tuning is one click away and remembered. This was not a preference: 576 plus 215 is 791 against a 620px slot, so something had to go under it, and the tuning was the honest candidate.

**Background Type tab (`wallpapers` | `color` | `gradient`).** A `Segmented` row since the panel pass (three exclusive kinds, one word each - benchmark section (b) point 2), replacing the hand-rolled `.e-seg` button row. Local `useState`, initialized from `doc.settings.background.kind` (`solid` -> `color`, `gradient` -> `gradient`, anything else -> `wallpapers`) so reopening the panel lands on the right tab. `image` and `video` fall into `wallpapers` on purpose: that is where the asset card lives, so a project reopened on an imported background shows the thing that is actually selected rather than an unrelated grid. Switching tabs alone never calls `onSaveSettings` - only picking an actual tile or colour does, so browsing has no side effect.

**Thumbnails.** One `backgroundThumbs()` call per mount, into local state. Rust renders the tiles once per process, so the refetch is a clone after the first time. A rejected call (no ffmpeg, an older backend) leaves the list empty and is swallowed: the rows still render their tiles, just without artwork, so nothing becomes unselectable and no error toast fires for a cosmetic miss.

- **`wallpapers` tab:** one `WallpaperRow` per group (`wallpaperGroups`), each labelled by the group itself - `Ribbons`, `Folds`, `Gradients`, `Metal`, `Scenic`, in whatever order the backend emitted, with `CLASSIC` (the legacy bundled `bg.jpg`, id `""`) leading the first one. Clicking a tile writes `{ kind: "mesh", mesh: id }`, so Classic (empty id) is exactly what every pre-library project already has. The rows are data-driven: a new wallpaper set ships by dropping JPEGs into `src-tauri/assets/backgrounds/wallpapers/`, with no change in this file. All six rows sit in ONE `.e-grp.e-rowstack` at 8px rather than six sections at 16, because they read as one library. This tab absorbed what used to be separate `image` and `video` tabs: `video` had no real backing at all (a coming-soon note only) and was dropped entirely, while `image` had been pared down to a single swatch back when `mesh` meant one hardcoded picture. It now has a real library behind it. No `"Presets"` heading: the tab name already says what the rows are.
- **Custom (the last row):** `BackgroundAssetCard` - the import tile leading the row, with a card for whatever is already imported beside it. It is a row in the same stack, not a section of its own, because it is one more way to choose a background. Importing writes `{ kind: info.kind, asset: info.rel_path }`; selecting the card re-asserts the asset's own kind; Remove writes `{ kind: "mesh", asset: null }` AFTER the file is deleted. Picking a wallpaper, colour or gradient never clears `asset`, which is what makes coming back to the card free.
- **`color` tab:** `COLOR_PRESETS` (RGB tuples, `backgroundPresets.ts`) - clicking one calls `onSaveSettings` with `background: { ...bg, kind: "solid", solid: c }` - plus a `ColorInput` beside them writing the same field, so the presets are a shortcut rather than the only choice. Real: renders via `export::scene::background::render(&Background::Solid(...))`.
- **`gradient` tab:** delegated to `GradientTab` - twelve backend-rendered preset tiles, two or three `ColorInput` stops, a middle-stop toggle, and a 0..360 angle slider. The presets now live in Rust (`settings::wallpapers::GRADIENT_WALLPAPERS`) and arrive with their stops attached, so the TypeScript copy in `backgroundPresets.ts` was deleted: one table, no drift. Three-stop presets became possible at the same time, since `Background::Gradient` gained an optional `mid`.

**Accent Colors, Corner Radius, Padding** - same controls, wired to `doc.settings.ui.accent` / `doc.settings.appearance.screen.{screen_radius,pad}`; Corner Radius and Padding are the "Frame" group's two-up row, and both groups plus Accent Colors sit under **More**.

**Background Blur** - `0..100%` slider writes `background.blur` (`0..1`). Applied ONCE to the static background buffer in Rust (`export::scene::background::blur`, a two-pass box blur), not per frame, so it stays cheap despite not being GPU-accelerated.

*Hidden on a video background.* Because it is a one-off pass over the static buffer, on `kind: "video"` it would only ever reach the asset's first frame. A control that visibly does nothing reads as broken, so the slider is replaced there by one quiet line saying why: "Blur applies to still backgrounds." Rust is unchanged either way - the still buffer (the fallback, and what the Rust preview commands return) is still blurred.

**Dim** - `0..80%` slider writes `background.dim` (`0..0.8`), a black overlay over whichever background actually has pixels. It sits beside Blur, outside the tab switch, because it applies to a wallpaper, an image and a video alike. Applied exactly once per side: in Rust for everything the backend rasterises, in `stage/stageBg.ts` for the preview's own video draw.

### Removed (this change, not carried forward as dead controls)

- **Frame Shadow slider** - there is no drop-shadow rendering in the export compositor at all (only the TS preview canvas draws a hardcoded, non-configurable shadow via `ctx.shadowBlur`). Wiring this for real would mean adding a shadow pass to both the GPU shader (`shader.wgsl`) and the CPU compositor - a new rendering subsystem, out of scope here. Flagged rather than faked; the existing cosmetic preview-only shadow is untouched (it was never tied to this dead slider anyway, so removing the slider doesn't change the preview's appearance).
- **Remove Background switch** - real background removal needs a segmentation/matting subsystem; no backend exists.

**Reset** - `DEFAULT_BG` (now in `backgroundPresets.ts`, beside the swatch tables, because this file is at its line budget) mirrors the Rust `BackgroundSettings::default()`, including `mesh: ""` (Classic), `gradient_mid: null`, `asset: null` and `dim: 0`, so pressing Reset lands on exactly what a fresh project has. Clearing `asset` clears the choice, not the file: the import stays in `<project>/background/` and is one click away again.

### Used by

- `src/editor/Editor.tsx` - the `tab === "background"` panel.
- `src-tauri/src/settings/background.rs` (`BackgroundSettings`) - the settings shape this panel edits.
- `src-tauri/src/export/scene/background.rs` (`build`) - turns the settings into the pixels this panel's choices produce.
- `src-tauri/src/settings/wallpapers.rs` - the wallpaper and gradient tables behind the rows.
- `src-tauri/src/export/preview/bg_thumbs.rs` (`background_thumbs`) - renders the tiles.
