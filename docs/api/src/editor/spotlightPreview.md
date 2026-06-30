# src/editor/spotlightPreview.ts

The live preview's spotlight: computes the spotlight strength at a time and draws the dim overlay on the canvas, mirroring the export so a spotlight region (or the global settings toggle) shows in the editor preview, not just in the render.

## spotlightAlpha

```ts
export function spotlightAlpha(effects: EffectRegion[], holds: HoldSpan[], settingsOn: boolean, ms: number): number
```

Spotlight strength `0..1` at output time `ms`: the max fade-in/out ramp (250ms, matching the export's `FADE_MS`) over any Spotlight `EffectRegion` AND any recorded hotkey-hold span (`HoldSpan`, from the `spotlight_holds` command) covering `ms`, unioned with the global `settingsOn` toggle. This mirrors the export's full union `s_alpha = max(settings, hold_alpha(actions), region_alpha)`, so a spotlight you held while recording now shows in the editor preview, not only editor-added regions. The private `ramp(start, end, ms)` helper computes one interval's ramp and is shared by both the region and hold loops.

## SpotParams

```ts
export interface SpotParams { dim: number; radius: number; feather: number }
```

The spotlight look from the recording's `clickfx` settings: `dim` (max darkness 0..1) and `radius` + `feather` (fractions of output height).

## SpotlightInput

```ts
export interface SpotlightInput { effects: EffectRegion[]; holds: HoldSpan[]; on: boolean; params: SpotParams }
```

What `drawPreview` needs to draw the spotlight: the editable regions, the recorded hotkey holds, the global toggle (`on`), and the params. `Stage` passes `null` when `clickfx.enabled` is false (no FX at all).

## drawSpotlight

```ts
export function drawSpotlight(ctx: CanvasRenderingContext2D, w: number, h: number,
  p: [number, number], alpha: number, sp: SpotParams): void
```

Draws the spotlight over the whole canvas with a soft hole at the cursor `p`, approximating the export's Classic spotlight (a colorless radial darken toward black): transparent within `radius * h`, reaching `dim * alpha` past `(radius + feather) * h`. No-op when `alpha <= 0`. Other modes (Vignette/Nebula/Halo/Blur/Breathing) and recorded hotkey-hold spotlights are not yet previewed.
