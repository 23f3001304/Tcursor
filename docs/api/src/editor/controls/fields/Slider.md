# src/editor/controls/fields/Slider.tsx

A custom range slider: pointer-drag track + spring thumb (Motion), and (as of Task 26) full
keyboard support per the WAI-ARIA slider pattern.

## snapToStep

```ts
export function snapToStep(raw: number, min: number, max: number, step: number): number
```

Snaps `raw` to the nearest `step`, clamped to `[min, max]`, with output decimal precision
matching `step`'s own (so e.g. `step=0.01` never produces `0.30000000000000004` from naive
floating-point arithmetic). Shared by the pointer-drag path (`updateValue`) and the keyboard path
(`sliderKeyValue` below) so both land on identical values for the same effective position.

## sliderKeyValue

```ts
export function sliderKeyValue(key: string, value: number, min: number, max: number, step: number): number | null
```

The value a key press should move the slider to, or `null` if `key` isn't one of the standard
slider keys. `ArrowRight`/`ArrowUp` and `ArrowLeft`/`ArrowDown` move by one `step`; `PageUp`/
`PageDown` move by 10 steps; `Home`/`End` jump to `min`/`max`. Every non-null result is already
passed through `snapToStep`.

## Slider

```tsx
export function Slider({ value, min, max, step, onChange, disabled, accentColor, ariaLabel, label, formatValue }: {
  value: number; min: number; max: number; step?: number; onChange: (v: number) => void;
  disabled?: boolean; accentColor?: string; ariaLabel?: string;
  label?: string; formatValue?: (v: number) => string;
}): JSX.Element
```

### Props

- `ariaLabel?: string` - (Task 26) sets `aria-label` on the `role="slider"` track. Every call
  site in the codebase passes this - a screen reader otherwise has no name for a bare slider.
- `label?: string` / `formatValue?: (v: number) => string` (render hygiene pass) - an optional
  `.e-fl` readout rendered ABOVE the track, as a sibling (via a `<>` Fragment - no extra wrapper
  DOM node, so it's layout-identical to the hand-rolled `<span className="e-fl">` markup every
  call site used before this prop existed). Sourced from the LIVE `shown` value (see "Commit
  debounce" below), not the committed `value` prop, so the number updates at full pointer rate
  during a drag instead of stepping at the (now debounced) commit rate - see "Optimistic local
  value" below for why that gap exists at all. `formatValue` renders `shown`; omitted, `shown` is
  shown raw. Omit `label` entirely to render nothing here (the caller keeps rendering its own
  label from `value`, as most non-numeric-readout callers still do - see "Not converted" below).

### Behavior (Task 26 additions)

The track div is `role="slider"`, `tabIndex={disabled ? -1 : 0}`, with `aria-valuemin`/
`aria-valuemax` mirroring the numeric props. Focus is shown via `.e-slider-track:focus-visible`
(`editor.css`), a `--e-focus`-colored outline - the same token used across the editor's other
focus rings, not a new color.

### Commit debounce + optimistic UI (render hygiene pass)

Most `onChange` callers ultimately fire an `apply_edit_op`/`save_edit` IPC round trip (often
cascading into further refetches - see `useEditorData.md`'s `previewBg`). Calling `onChange` on
every `pointermove` - as this component used to - turned a single slider drag into 60-144
IPC round trips a second (perf sweep F#2/#5/#6). `Slider` now debounces the COMMIT while keeping
the visible thumb fully responsive:

- **Optimistic local value.** A pointer interaction sets local state `dragValue` immediately; the
  thumb/fill/`aria-valuenow`/`label` readout are all computed from `shown = dragValue ?? value`,
  so the UI tracks the pointer at full rate regardless of how slow `onChange`'s own round trip is.
- **Clear-on-change, not clear-on-equality (fix round 1).** `dragValue` clears via the shared
  `shouldClearOverride` (`../util/overrideClear.ts`) once the `value` PROP changes away from
  `settledValueRef.current` (a snapshot taken at the moment the current gesture - a drag or a
  keypress - began), not once it happens to land back on the EXACT value `dragValue` holds. The
  original exact-equality version could get stuck showing a value the doc doesn't hold forever:
  it could never resolve a commit the backend clamped or rejected outright (`value` would simply
  never equal `dragValue`), and was deaf to an undo/Reset landing mid-wait (that changes `value`
  to something else entirely, which equality against `dragValue` never recognized as "stale now"
  either). See `overrideClear.md` for the shared predicate's full reasoning (including its one
  structural limit: a commit that fails so silently `value` never changes AT ALL still can't
  self-resolve - there's no external signal for that case to key off of).
- **Debounced commit, `flush()` on release.** The actual `onChange(v)` call is wrapped in
  `debounce(...)` (`../util/debounce.ts`, `COMMIT_DEBOUNCE_MS = 80`) - a drag calls the debounced
  wrapper on every move, but it only actually invokes `onChange` at most once every 80ms. Release
  (`endDrag`, wired to `pointerup`/`pointercancel`/`lostpointercapture` - see below) `flush()`es
  immediately, so the drag's FINAL value always lands without waiting out the trailing window.
  `onChange` itself is read through a ref (`onChangeRef`), so the debounced wrapper never needs
  recreating just because the caller passed a fresh inline arrow (as most callers do).
- **`pointercancel`/`lostpointercapture` also end the gesture (fix round 1).** `endDrag` is wired
  to all three of `onPointerUp`/`onPointerCancel`/`onLostPointerCapture`, not just `onPointerUp` -
  a cancelled sequence (palm rejection, a system gesture stealing the pointer) or a lost-capture
  notification never fires `pointerup` at all, which previously left `dragging` stuck `true`
  forever and permanently blocked the clear-on-change effect from ever running again for that
  slider. `endDrag` is idempotent, so it's safe that `lostpointercapture` also fires right after a
  normal release's own `releasePointerCapture` call.
- **Keyboard bypasses the debounce.** `sliderKeyValue`'s result commits immediately (`cancel()`s
  any pending debounce, calls `onChange` directly) - a discrete key press has nothing to coalesce.
- **Unmount safety.** A pending commit still `flush()`es on unmount (switching panels,
  deselecting mid-drag), so a value the user actually dragged to is never silently dropped.

This mirrors the exact pattern `Stage.tsx`'s reticle drag uses for the same reason (see
`Stage.md`'s "Reticle drag debounce") - local optimistic state + a shared `debounce()` + a
release-time flush + the same shared `shouldClearOverride`.

### Not converted to `label`/`formatValue`

`ExportDialog.tsx`'s CRF slider only - it uses a different label pattern entirely (`.e-export-row`
+ a plain `<label>`, not `.e-fl`) with a conditional compound string (`"Quality"` vs `"Quality
(CRF 22)"`), and is local-`useState`-only (no IPC at all), so the lag `label` fixes doesn't apply
there in the first place.

`EffectInspector.tsx`'s `OverrideField` (fix round 2) and `AudioPanel.tsx`'s Mic Sync Offset
slider (fix round 2) WERE initially left unconverted for a structural reason - a value readout
that needed to sit somewhere other than immediately above the track (a `.e-switchrow` alongside a
`Switch`, and a `.e-hintrow` respectively) - but both are now converted anyway, with a small
supporting layout change each (see `EffectInspector.md`'s `OverrideField` section and
`AudioPanel.md`'s "Live readout" note) rather than staying unconverted, since leaving the lag in
place wasn't an acceptable fix for those findings.

### Editable value readout (panel pass, 2026-09-13)

`label` no longer renders `{label} <b>{formatValue(shown)}</b>`. It renders `SliderValue`
(`SliderValue.md`): the same one-row `.e-fl`, name left and value right in tabular figures, but the
value is **click-to-type** - Enter commits, Esc reverts, blur commits, and a typed number is snapped
and clamped exactly like a dragged one. Rationale in that file: at 320px a track cannot reliably
reach an exact degree or pixel, and the panels are full of ranges where a user knows the number they
want.

`Slider`'s props did not change, so every call site is untouched. Internally the keyboard path and
the typed path now share one `commitNow(next)` - snapshot the settled value, paint optimistically,
cancel any pending drag debounce, call `onChange` straight away - because both are discrete and have
nothing to coalesce.

### Look

The rail is a groove one plane DOWN (`--e-bg`, 4px) rather than a raised bar, and the thumb is a
plain `--e-fg` disc with a shadow and no border - surfaces read by lightness, not strokes.
`accentColor` still defaults to `--e-fg`, so a slider's fill stays neutral and the panels' one accent
(`--e-primary`) is left to mark selected state. The rules moved from `editor.css` to
`controls/controls.css`.

**Hit strip: 24px (usability pass, 2026-09-13).** The `role="slider"` div was 20px tall. That div
IS the control's hit target - the 4px rail inside it is only paint - so it sat under the 24px floor
the pass set for anything clickable, and it is 24 now. Nothing about the rail, the thumb or the
geometry changed; the strip around them simply got 2px taller on each side.

**Two-up (`.e-two`).** Several panels now put two short sliders side by side, roughly 139px each.
Nothing in `Slider` needs to know: the track is `width: 100%` and `SliderValue`'s `.e-fl-name`
ellipsises. The rule for whether a pair qualifies is the NAME's length, and it lives with the
`.e-two` class in `panels.css`.
