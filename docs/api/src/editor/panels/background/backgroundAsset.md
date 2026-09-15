# src/editor/panels/background/backgroundAsset.ts

Reading an imported background asset for the panel: its name, its kind, the line under its name, and where its thumbnail lives. Pure, so `BackgroundAssetCard` stays a thin render.

Two of these mirror Rust (`settings::bg_asset`): the extension table and the thumbnail path. They are duplicated rather than fetched because the card must be able to draw a tile before any IPC resolves - but both are one-liners whose Rust twin is named in the comment, and both are pinned by tests on each side.

## assetFileName

```ts
export function assetFileName(rel: string): string
```

The file name out of a project-relative asset path (`background/my clip.mp4` -> `my clip.mp4`).

## assetKindOf

```ts
export function assetKindOf(rel: string): "image" | "video" | null
```

Which `BackgroundKind` this asset is, by extension. Mirrors `bg_asset::asset_kind_for`, case-insensitively. `null` for anything unrecognised, which is what disables the card's own select action rather than writing a kind the renderer cannot honour.

## assetSubtitle

```ts
export function assetSubtitle(info: BackgroundAssetInfo | null | undefined): string
```

The line under the file name: `"Video 1920x1080, 4.2s"`, `"Image 2560x1440"`.

The three states are deliberate. `undefined` means "still asking" and prints nothing, rather than flashing a wrong answer for one frame. `null` means the probe RESOLVED and the file is gone, and says so: `"File missing"`. A real `info` whose dimensions came back as zeros (a probe that failed on a file that exists) drops the size instead of printing `0x0`.

## thumbRel

```ts
export function thumbRel(rel: string): string
```

Where the asset's thumbnail lives: `background/clip.mp4` -> `background/.thumbs/clip.mp4.jpg`. Mirrors `bg_asset::thumb_rel`, including keeping the full file name so `a.png` and `a.mp4` get separate thumbs.

### Behaviors

- `assetFileName` handles a nested path, a bare name, and an empty string.
- `assetKindOf` mirrors the Rust table for both cases and is `null` for an unknown extension and an empty path.
- `assetSubtitle` prints a duration only when there is one, says "File missing" for a resolved `null`, stays silent while the probe is in flight, and drops an unprobed dimension.
- `thumbRel` matches the Rust derivation, with and without a folder.

### Used by

- `src/editor/panels/background/BackgroundAssetCard.tsx` - all four.
- `src-tauri/src/settings/bg_asset.rs` - the Rust side of the two mirrored ones.
