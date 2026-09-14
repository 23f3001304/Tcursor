# src/editor/stage/stageBg.ts

The preview's background: what the stage is handed, what it holds, and how one frame of it is drawn.

Like `previewCanvas.ts`, everything drawn here is EXPORT pixels and is therefore theme-independent: the dim is the same `out = src * (1 - dim)` black overlay Rust applies, and the `#2c2c42 -> #131318` gradient is the placeholder that stands in for a background PNG that has not landed yet - both would be wrong if they turned with the editor's light theme, because neither is chrome. The frame's surrounding chrome (the stage well and the frame's own ground) IS themed, via `--e-stage-well` and `--e-frame` in `stage.css`.

**Two paths, and the split is the whole design.** For everything STATIC (wallpaper, colour, gradient, an imported still) the backend already returns the finished background as a PNG at the output size - cover-fitted, blurred and DIMMED - so the preview blits it exactly as it always has, and re-applying any of that here would double it. For a VIDEO or GIF the backend cannot hand over sixty images a second, so this file draws the moving asset itself: cover-fit with the same rect math the webcam PiP uses, then the dim as a black overlay.

**Parity for a moving background is "the same frame within one output frame", not pixel-identical.** The preview resamples through the browser's decoder and the export through ffmpeg's `-r`, so a single-frame difference at a cut is expected and is not a bug. Everything about WHICH frame is shown is shared: both sides count from the recording's own frame 0 and wrap by the asset's duration.

## StageBg

```ts
export interface StageBg {
  url: string; assetUrl: string; assetPath: string; kind: BackgroundKind; dim: number;
}
```

What `Editor` hands the stage. ONE prop, deliberately: `Stage`, `useCompositeLoop` and `drawPreview` all kept their existing signatures when video backgrounds landed because this replaced the old `bgUrl: string` rather than joining it (those three files sit within a few lines of the 200-line cap).

- `url` - `preview_bg`'s data URL, the static background, already dimmed in Rust.
- `assetUrl` - the imported asset as an `asset://` URL, or `""` when the active kind does not draw one (see `bgAssetUrl`).
- `assetPath` - `settings.background.asset`, project-relative, for the `.gif` decision.
- `dim` - applied here ONLY on the branch that paints pixels itself.

## StageBgState

```ts
export interface StageBgState {
  bg: StageBg; img: HTMLImageElement | null; video: HTMLVideoElement | null;
  gif: GifFrames | null; playing: boolean;
}
```

The live elements behind a `StageBg`, owned by `useStageInvalidation` and read every tick through a ref. The `<video>` is created with `document.createElement` and never mounted - `drawImage` needs no DOM node, and an unmounted element cannot be laid out, styled, or accidentally shown. `playing` is latched from the transport so the draw can tell a scrub from playback without re-rendering.

## coverRect

```ts
export function coverRect(sw: number, sh: number, dw: number, dh: number): [number, number, number, number]
```

The largest centred sub-rect of a `sw`x`sh` source with the destination's aspect, as `drawImage`'s four source arguments. Cover-fit: the mismatched axis is CROPPED, never squashed, so a 16:9 loop behind a 9:16 export fills the frame instead of letterboxing. The same fit `ffio::decode_file_cover` gives a still and `bg_decode_args` asks ffmpeg for - which is what makes preview and export frame an asset identically.

Returns the whole source when either side is degenerate, so a `<video>` whose metadata has not arrived cannot produce `NaN` coordinates.

## loopMs

```ts
export function loopMs(tMs: number, durMs: number): number
```

The playhead wrapped into a loop of `durMs`. `0` when the duration is not known yet (metadata still loading), so early frames show frame 0 rather than `NaN`.

`0` for a playhead before the clip starts too: that is not a loop position, and wrapping it backwards would answer "before the beginning" with the END of the loop. `gifIndexAt` clamps the same way, so the two paths agree at the edges.

## isGif

```ts
export function isGif(assetPath: string): boolean
```

Is this asset a GIF? The settings model calls a GIF a `video` (one export decode path for both), but a `<video>` element cannot play one, so the PREVIEW's decode path is chosen by extension here.

## bgAssetUrl

```ts
export function bgAssetUrl(folder: string, asset: string | null | undefined, kind: BackgroundKind,
                           toSrc: (p: string) => string): string
```

The asset's URL for the CURRENT kind, or `""`. `toSrc` is `fileSrc` (Tauri's `convertFileSrc`), injected so this stays unit-testable.

*Why it can be empty while `asset` is set:* the asset is deliberately kept in the settings while a wallpaper or colour is showing, so re-selecting it needs no re-import - but it must not be LOADED then, or a hidden `<video>` would sit there decoding a background nobody can see.

The stored path is forward-slashed (portable); this is where it becomes a platform path.

## drawBackground

```ts
export function drawBackground(ctx: CanvasRenderingContext2D, w: number, h: number,
                               st: StageBgState | null, tMs: number): void
```

Draw the background for one preview frame. A `video` kind with something decoded takes the moving path and then paints `rgba(0,0,0,dim)` over it - `out = src * (1 - dim)`, exactly `background::apply_dim`'s formula in Rust. Everything else blits the still PNG, or the same gradient placeholder the preview has always used while it loads.

## drawMoving

```ts
function drawMoving(ctx, w, h, st: StageBgState, tMs: number): boolean
```

One frame of a GIF (by `gifIndexAt`) or of a video, cover-fitted. `false` when nothing is decoded yet, which makes `drawBackground` fall back to the still background rather than flashing a gap.

For a video: paused, it seeks to the playhead; playing, it lets the element run at 1x beside the screen video and corrects only real drift (150ms, the same tolerance the webcam re-sync uses). Seeking every frame during playback would stall the decoder and stutter the picture.

### Behaviors

- `coverRect` covers by cropping the long axis, is identity at a matching aspect, and is degenerate-safe.
- `loopMs` wraps, is 0 for an unknown or `NaN` duration, and is 0 before the clip starts.
- `isGif` is case-insensitive and false for an empty path.
- `bgAssetUrl` is empty for a kind that does not draw an asset (even when one is remembered) and for no asset.

### Used by

- `src/editor/stage/previewCanvas.ts` (`drawPreview`) - calls `drawBackground` once per frame.
- `src/editor/stage/useStageInvalidation.ts` - owns the `StageBgState` and keeps the video in step with the transport.
- `src/editor/hooks/useEditorData.ts` - builds the `StageBg` from `doc.settings.background`.
- `src-tauri/src/export/scene/background.rs` (`apply_dim`) - the formula this mirrors.
