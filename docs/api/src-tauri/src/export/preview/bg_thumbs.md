# src-tauri/src/export/preview/bg_thumbs.rs

Thumbnails for the editor's background picker. Every tile is rendered by the SAME code the export uses (`ffio::decode_image_cover` for the bundled wallpapers, `scene::background::render` for the gradients), so a tile is a true 96x54 miniature of the background it applies - not a CSS lookalike that drifts from the render as the two are edited apart.

Built once per process into a `OnceLock`. Every bundled wallpaper costs an ffmpeg decode, so only the first call pays for them; every later call clones an in-memory `Vec`.

## GradientStops

```rust
#[derive(Serialize, Clone)]
pub struct GradientStops { pub from: [u8; 3], pub mid: Option<[u8; 3]>, pub to: [u8; 3], pub angle_deg: f32 }
```

A gradient tile's actual stops, carried alongside its thumbnail. *Why the panel gets the stops rather than just an id:* selecting a gradient preset writes `gradient_from`/`gradient_mid`/`gradient_to`/`gradient_angle_deg` into settings, so the panel needs the values. Shipping them here keeps `settings::wallpapers::GRADIENT_WALLPAPERS` the only copy of the table - the alternative (a mirrored preset list in TypeScript) drifts silently the moment either side is edited.

## BackgroundThumb

```rust
#[derive(Serialize, Clone)]
pub struct BackgroundThumb {
    pub id: String, pub name: String, pub kind: String, pub png_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub gradient: Option<GradientStops>,
}
```

One picker tile.

- `id` - the wallpaper id the panel writes into `BackgroundSettings.mesh`, or the gradient preset's id (used only for selection state).
- `name` - the tile's label.
- `kind` - the `BackgroundKind` this tile applies: `"mesh"` or `"gradient"`.
- `group` - the picker section: a wallpaper's own `Wallpaper.group`, or `"Presets"` for the procedural gradients. *Why `"Presets"` and not `"Gradients"`:* `Gradients` is one of the wallpaper GROUPS (the `gradient-` image set), and two different things under one label in the same flat list is a trap waiting for the first caller that groups without filtering by `kind` first.
- `png_base64` - the 96x54 tile, base64 PNG. EMPTY when the decode failed (no ffmpeg on the machine). *Why an empty string rather than dropping the entry:* the wallpaper is still perfectly selectable and renderable at export time through a different code path, so hiding the tile would take away a working choice; the panel paints a plain swatch instead.
- `gradient` - present on gradient tiles only; omitted from the JSON entirely for wallpapers.

## background_thumbs

```rust
#[tauri::command]
pub async fn background_thumbs() -> Result<Vec<BackgroundThumb>, String>
```

Every background preset as a thumbnail, in panel order: the bundled wallpapers in their group order (`WALLPAPERS`) then the twelve gradient presets (`GRADIENT_WALLPAPERS`).

*Why `async` + `spawn_blocking`:* the first call decodes every bundled JPEG through an ffmpeg subprocess. A sync `#[tauri::command] fn` runs that inline on the main thread and freezes the window for its duration - the same mechanism `preview_bg` and `thumbs::ensure_thumbs` were moved off for.

### Returns

The cached `Vec<BackgroundThumb>`, cloned. `Err` only if the blocking task itself panicked or was cancelled - a failed wallpaper decode is reported per tile (empty `png_base64`), never as a whole-command failure.

### Behaviors

- `every_preset_gets_a_tile_in_panel_order` - one tile per table entry, the wallpapers first and the gradients last, each matching its table's own order and carrying a non-empty id, name and group. Counted from `WALLPAPERS.len()` rather than a literal, since the wallpaper set grows.
- `a_gradient_tile_is_the_real_render_and_carries_its_stops` - a gradient tile's PNG is byte-identical to rendering that gradient directly, its `gradient` field holds the table's own values, and a wallpaper tile has no `gradient` at all.
- `a_failed_decode_still_lists_the_tile` - a `None` buffer yields an empty `png_base64` on a tile that still carries its id, rather than a missing entry.
- `wallpaper_tiles_decode_and_the_cache_is_built_once` - the `OnceLock` hands back the same allocation every call, and (when ffmpeg is present) every bundled wallpaper really does produce a PNG.

### Used by

- `src/lib/ipcPreview.ts` (`backgroundThumbs`) - the TypeScript binding.
- `src/editor/panels/BackgroundPanel.tsx` - fetches once per mount and feeds both the Wallpapers and Gradient grids.
