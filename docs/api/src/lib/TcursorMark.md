# src/lib/TcursorMark.tsx

The TCursor brand mark ("The Print"): a transparent, wave-only glyph for in-app use, distinct from the tiled app icon (`src-tauri/icons/source.svg`). Its geometry mirrors `public/brand/tcursor-mark.svg` exactly (same path data, viewBox, and dot) so the two never drift.

## TcursorMark

```tsx
export function TcursorMark({ size = 20, dotColor = "var(--e-primary, #ef4444)", state = "idle", pct }: {
  size?: number; dotColor?: string; state?: MarkState; pct?: number;
}): JSX.Element
```

Renders the wave + REC-dot brand mark inline.

### Props

- `size?: number` - both `width` and `height` of the rendered `<svg>`, in pixels. Defaults to 20. *Why one number for a non-square mark:* the `viewBox` is wider than it is tall (194x83), so setting equal `width`/`height` lets the SVG's default `preserveAspectRatio` ("meet") fit the artwork inside a `size` x `size` box without stretching or overflowing a fixed-size container - required at both call sites (`.e-mark` in `src/editor/shell/TopBar.tsx`, `.brand-mark` in `src/hud/Hud.tsx`), which are square badges that don't clip overflow.
- `dotColor?: string` - CSS color (or `var(...)`) for the REC dot's `fill`, unless `state` overrides it (see `dotTint` below). Defaults to `"var(--e-primary, #ef4444)"`, correct as-is for the editor site. The HUD call site overrides it to `"var(--accent, #ef4444)"` so the dot tracks the user's chosen accent color (`applyTheme.ts` writes `--accent` from Preferences) instead of a fixed red that would drift from the rest of the HUD's theming.
- `state?: MarkState` (Task 39 - "the living brand") - `"idle" | "recording" | "exporting" | "directing"`, default `"idle"`. Idle renders byte-for-byte what this component always rendered (a single static wave path, no `motion.g` wrapper) - every pre-Task-39 caller that doesn't pass `state` is unaffected. Drives `brandWave.ts`'s pure `flowSeconds`/`dotPulses`/`dotTint` mappings (see `brandWave.md`). Callers gate `state` itself by the `ui.animated_brand` setting BEFORE passing it in (this component has no Settings/IPC access, by design - see Notes).
- `pct?: number` - export percent-complete (0..100), only meaningful when `state === "exporting"` (see `flowSeconds`). Threaded straight from `TopBar`'s existing `pct` prop.

### Behavior

**Colors.** The wave path is `stroke="currentColor"` - it inherits whatever CSS `color` the container sets, in EVERY state (the brief's "the wave stays currentColor" rule - only the dot ever retints). Both call sites now set `color` explicitly on the badge, not the mark: `.e-mark` (`src/editor/editor.css`) sets `color: #fff` (the editor is unconditionally dark-themed, so white is always correct); `.brand-mark` (`src/hud/hud.css`) sets `color: var(--fg)` (the HUD toggles light/dark, so the wave must flip between near-black and near-white with the theme rather than staying a fixed white). The REC dot's fill is `dotTint(state) ?? dotColor` - `"directing"` is the only state `dotTint` overrides (to `--e-ai`); every other state keeps the caller's own `dotColor`.

*Why the badges themselves changed from a solid red fill to a soft dark surface + border (`.e-mark`: `background: var(--e-soft); border: 1px solid var(--e-border)`; `.brand-mark`: `background: var(--hover); border: 1px solid var(--line)`):* the original badges filled with the same red as the REC dot (`var(--e-primary)` / `var(--accent)`), which made the dot disappear into its own background - the mark rendered as "just a wave" in the shipped app. Both replacement tokens are each site's existing "one step more elevated than the base surface" token (mirroring the editor's `--e-bg -> --e-surface -> --e-card -> --e-soft` elevation ramp), so the tile now reads as a small dark chip - closer to the tiled app icon's dark tile - behind a light wave and a colored dot, in both of the HUD's themes.

**Geometry.** `viewBox="-33 22 194 83"` is a tight crop (~4px margin) around the union of the wave polyline's stroked bounds (`y` in [35, 101] from a `yMid` of 68, amplitude 18, and half the 30px stroke width) and the REC dot's bounds (`cx=103 cy=33 r=7`). The path data is the exact, unmodified "no offset, white" wave layer from the approved icon construction - not resampled or regenerated.

**Flow animation (Task 39).** When `flowSeconds(state, pct)` (after the reduced-motion gate) is `> 0`, swaps the single static `<path>` for a `motion.g` wrapping `WAVE_COPIES` - five copies of the same path, at x-offsets `[-2λ, -λ, 0, λ, 2λ]` (`λ = WAVE_LAMBDA`) - animated `x: [0, -λ]`, linear, `repeat: Infinity`, over `flowSeconds` seconds. *Why five copies, not one or two:* the animated range is one full λ, and the viewBox (194 units) is wider than λ (88) - at ANY point mid-animation, the visible window needs continuous wave coverage across roughly `viewBox width + λ` (~282 units) of pattern space, not just at the two loop endpoints; two copies leave a visible gap at one end of the loop (verified by hand against the viewBox bounds at dispatch - see `brandWave.ts`'s `WAVE_LAMBDA` doc). Five is generous margin for a handful of cheap extra `<path>` elements. When `seconds === 0` (idle, reduced-motion, or the setting off), only the single native-position path renders - identical DOM to before Task 39.

**Dot pulse (Task 39).** When `dotPulses(state)` (after the reduced-motion gate) is true, the `<circle>` becomes a Motion spring: `animate={{ scale: 1.2, opacity: 0.7 }}`, `transition={{ type: "spring", stiffness: 300, damping: 10, repeat: Infinity, repeatType: "reverse" }}` - springs to the dimmer/bigger state and back, indefinitely. `style={{ transformBox: "fill-box", transformOrigin: "center" }}` centers the scale on the circle's own bounding box rather than the SVG's `(0,0)` origin, so it doesn't need the exact `cx`/`cy` hardcoded into a pixel `transform-origin`.

**Reduced motion.** `useReducedMotion()` (now `src/lib/wave/ui/useReducedMotion.ts` - it was local to this file until the wave motif grew past the brand mark and four other components needed the same answer) checks `matchMedia("(prefers-reduced-motion: reduce)")` directly and listens for live changes, independent of any Motion `MotionConfig` ancestor, since the editor tree has none (only the HUD wraps its tree in `<MotionConfig reducedMotion="user">`) and this mark needs to honor the OS preference wherever it mounts. When reduced, `seconds` and `pulsing` are forced to their off state regardless of `state`.

### Notes

- No tile, no terraces, no shadow copy - this is deliberately the reduced, transparent-background variant of the full app icon (see `src-tauri/icons/source.svg` for the tiled four-layer version, and `src-tauri/icons/icon-rec.png` for its own REC-lit variant, used by the dynamic Windows icon - `src-tauri/src/win/sys/brand_icon.rs` - a SEPARATE mechanism from this component).
- `stroke-width` is `30` in viewBox units, so it scales down proportionally with `size` like the rest of the artwork - there is no separate stroke-width prop.
- This component deliberately has no Settings/IPC dependency - the `ui.animated_brand` feel-knob gate lives at each call site (`src/editor/Editor.tsx` computes `brandState` from `doc.settings.ui.animated_brand`; `src/hud/Hud.tsx` tracks its own `animatedBrand` state from `getSettings`/`Preferences`), keeping this shared component a plain, dependency-free presentational piece.
