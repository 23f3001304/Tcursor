# src/editor/panels/BackgroundPanel.tsx

The Background rail panel: frame padding/corner radius (pre-existing), accent color (pre-existing), and - as of this change - a REAL background type/color/gradient/blur, all backed by `doc.settings.background` (`settings::background::BackgroundSettings` in Rust). Previously the type selector, presets, blur, frame-shadow, and remove-background controls were entirely disconnected from the renderer (a fixed background image/gradient was hardcoded in `FrameRenderer::new`, unconditionally, regardless of anything in this panel).

## BackgroundPanel

```tsx
export function BackgroundPanel({ doc, onSaveSettings, onClose }: {
  doc: EditDoc; onSaveSettings: (nextSettings: EditDoc["settings"]) => void; onClose: () => void;
}): JSX.Element
```

### Behavior

**Background Type tab (`image` | `video` | `color` | `gradient`).** Local `useState`, initialized from `doc.settings.background.kind` (`solid` -> `color`, `gradient` -> `gradient`, `mesh` -> `image`) so reopening the panel lands on the right tab. Switching tabs alone never calls `onSaveSettings` - only picking an actual preset does, so browsing the unsupported `image`/`video` tabs has no side effect.

- **`color` tab:** `COLOR_PRESETS` (RGB tuples, `backgroundPresets.ts`) - clicking one calls `onSaveSettings` with `background: { ...bg, kind: "solid", solid: c }`. Real: renders via `export::scene::background::render(&Background::Solid(...))`.
- **`gradient` tab:** `GRADIENT_PRESETS` (2-stop RGB + angle, `backgroundPresets.ts`) - clicking sets `kind: "gradient"` plus `gradient_from`/`gradient_to`/`gradient_angle_deg`. *Why only 2-stop presets, when the old (removed) preset list had some 3-stop CSS gradients:* the Rust `Background::Gradient` variant only supports two stops - a 3-stop swatch would look different from what actually renders, so the preset data was curated down to what the backend can reproduce exactly, rather than shipping a swatch that lies about the result.
- **`image` tab:** shows exactly ONE real preset, "Default" (sets `kind: "mesh"`, the bundled background image, today's original look) - plus a "Custom image backgrounds are coming soon" note. The previous build's 12 Unsplash-hosted preset thumbnails and "Upload Custom" button (which only called `alert(...)`) were removed: fetching remote images from a local capture app has no backend support and would have been network calls with no real effect.
- **`video` tab:** coming-soon note only, no presets. No default video background exists (unlike `image`'s bundled mesh).

**Accent Colors, Corner Radius, Padding** - unchanged from before this change (already wired to `doc.settings.ui.accent` / `doc.settings.appearance.screen.{screen_radius,pad}`).

**Background Blur** - now real: `0..100%` slider writes `background.blur` (`0..1`). Applied ONCE to the static background buffer in Rust (`export::scene::background::blur`, a two-pass box blur), not per frame, so it stays cheap despite not being GPU-accelerated.

### Removed (this change, not carried forward as dead controls)

- **Frame Shadow slider** - there is no drop-shadow rendering in the export compositor at all (only the TS preview canvas draws a hardcoded, non-configurable shadow via `ctx.shadowBlur`). Wiring this for real would mean adding a shadow pass to both the GPU shader (`shader.wgsl`) and the CPU compositor - a new rendering subsystem, out of scope here. Flagged rather than faked; the existing cosmetic preview-only shadow is untouched (it was never tied to this dead slider anyway, so removing the slider doesn't change the preview's appearance).
- **Remove Background switch** - real background removal needs a segmentation/matting subsystem; no backend exists.

### Used by

- `src/editor/Editor.tsx` - the `tab === "background"` panel.
- `src-tauri/src/settings/background.rs` (`BackgroundSettings`) - the settings shape this panel edits.
- `src-tauri/src/export/scene/background.rs` (`build`) - turns the settings into the pixels this panel's choices produce.
