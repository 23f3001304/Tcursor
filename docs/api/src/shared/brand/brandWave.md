# src/shared/brand/brandWave.ts

Pure state -> animation-config mapping for the living `TcursorMark` (Task 39). Kept separate from `TcursorMark.tsx` itself so the mapping is unit-testable without touching React/Motion at all - see `brandWave.test.ts`.

## MarkState

```ts
export type MarkState = "idle" | "recording" | "exporting" | "directing";
```

The mark's four animation states. `idle` is the default and today's plain static rendering. `recording`/`exporting`/`directing` each drive a different combination of `flowSeconds`/`dotPulses`/`dotTint` below.

### Used by

- `src/shared/brand/TcursorMark.tsx` - the `state` prop.
- `src/editor/Editor.tsx` - computes `brandState` (exporting/directing/idle, gated by `doc.settings.ui.animated_brand`) for `TopBar`.
- `src/hud/Hud.tsx` - computes `recording`/idle (gated by its own `animatedBrand` state) for the titlebar mark.

## WAVE_LAMBDA

```ts
export const WAVE_LAMBDA = 88;
```

The wave path's horizontal repeat period, in the same SVG user units as `TcursorMark`'s `WAVE_D`. A clipped translate by exactly this much loops seamlessly (verified against the path's own geometry at dispatch). `TcursorMark` tiles enough copies of the path, spaced one `WAVE_LAMBDA` apart, to cover the viewBox across the WHOLE animated range (not just its two endpoints) - see `TcursorMark.md`'s flow-animation note for why five copies.

## flowSeconds

```ts
export function flowSeconds(state: MarkState, pct?: number): number
```

Seconds for one full λ-wide flow loop in `state`.

### Inputs

- `state: MarkState` - the mark's current state.
- `pct?: number` - export percent-complete (0..100), defaults to `0`. Only read when `state === "exporting"`.

### Returns

- `"idle"` -> `0` (no animation - the caller renders the plain static mark).
- `"exporting"` -> linearly interpolated from `3` seconds/λ at `pct = 0` down to `0.8` seconds/λ at `pct = 100`, clamped to that range for any `pct` outside 0..100 (so a stale or out-of-range value never produces a nonsensical speed). *Design pass:* the brief asked for pace mapped from export progress; a direct `pct -> pace` curve was chosen over measuring a literal frame-to-frame percent velocity, which would need extra cross-render state (a previous-pct + previous-timestamp ref) for a purely cosmetic knob - the value-based curve still delivers the intended feel ("picking up speed" as the export nears done) with a pure, trivially-testable function.
- `"recording"` / `"directing"` -> a fixed `2` seconds/λ, regardless of `pct`.

### Behaviors

- `idle` never animates regardless of `pct`.
- `recording`/`directing` flow at a fixed 2s/λ regardless of `pct`.
- `exporting` interpolates 3s (0%) -> 0.8s (100%), including the midpoint (50% -> 1.9s).
- `exporting` clamps `pct` outside `0..100` to the nearest end of the range.
- `exporting` with `pct` omitted defaults to `0` (slowest pace).

## dotPulses

```ts
export function dotPulses(state: MarkState): boolean
```

Whether the REC dot should pulse (a repeating Motion spring on scale + opacity, see `TcursorMark.md`) in `state`.

### Returns

`true` only for `"recording"`; `false` for every other state.

## dotTint

```ts
export function dotTint(state: MarkState): string | null
```

The dot's color override for `state`, or `null` to keep the caller's own `dotColor` prop.

### Returns

`"var(--e-ai)"` for `"directing"`; `null` for every other state. The wave itself always stays `currentColor` regardless of `state` - only the dot ever retints, so the mark never stops reading as "the same mark", just differently animated.
