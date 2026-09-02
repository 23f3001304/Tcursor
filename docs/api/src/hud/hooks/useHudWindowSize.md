# src/hud/hooks/useHudWindowSize.ts

Owns the HUD's OS window sizing while the bar (not a Settings/Preferences panel) is showing: keeps the window tall enough for an open dropdown's menu, content-fit narrow while recording or saving (D5, item 3), fixes the cold-launch off-centre bug (L1), and re-normalizes the window when the OS reports a DPI/scale-factor change so the bar's `width:100%` CSS (hud.css) never ends up wider than the window's actual physical bounds. Extracted out of `Hud.tsx` to keep that file under the project's 200-line cap - no behavior changed in the move.

## WIDTH

```ts
export const WIDTH = 980
```

Fixed horizontal size of the HUD bar window while idle; only its height varies, with whether a dropdown menu is open. Also the width `Hud.tsx`'s panel-morph transitions (`openPanel`/`restoreBar`, which own the window size themselves while a Settings/Preferences panel is open or animating) animate back out to once a recording/save (or a panel visit) ends.

### Used by

- `src/hud/hooks/useHudWindowSize.ts` - `hudLogicalSize` builds on it.
- `src/hud/Hud.tsx` - `openPanel`/`restoreBar`'s `morphWindow` calls.

## RECORDING_WIDTH

```ts
export const RECORDING_WIDTH: number  // 538
```

Horizontal size of the bar window while recording (D5, user-reported 2026-09-02: the idle 980px window used to be kept for the recording pill too, ballooning the invisible always-on-top hit-blocking slack around a much narrower actual content width). Computed, not eyeballed: a sum of named constants each traceable to a real `hud.css` value (`.row`'s padding/gap, `.grip`, `.camtoggle`, `.recmeter`'s fixed-width `.wave`, `.timer`'s `min-width`, `.paused-chip`) plus a couple of measured (not CSS-exact) text-width terms for variable labels (the paused chip's "PAUSED", the Pause/Resume button, the Stop button) and a small safety margin covering that imprecision. Sized for the row's WIDEST state - paused, with the "Resume" label rather than "Pause" - so a pause/resume toggle mid-recording never triggers a second resize. Recomputed for gate-feedback item 2 (2026-09-02) when `.recmeter`'s `.wave` shrank from 80px to 63px (the compact-meter redesign) - was 555, now 538.

### Used by

- `src/hud/hooks/useHudWindowSize.ts` - `hudLogicalSize` returns it while `mode === "recording"`.
- `src/hud/hooks/useHudWindowSize.test.ts` - asserts it stays narrower than `WIDTH` and wider than `SAVING_WIDTH`.

## SAVING_WIDTH

```ts
export const SAVING_WIDTH: number  // 284
```

Horizontal size of the bar window while saving/preprocessing (item 3, user-reported 2026-09-02: "Saving… X%" fell back to the idle 980px window around ~300px of actual content, because `mode` never distinguished "saving" from "idle" - `recording` is already `false` by the time `saving` goes `true`, see `useRecordingFlow.ts`'s `finish`). Same computed-not-eyeballed methodology as `RECORDING_WIDTH`, but for the SAVING row's own (shorter) content: item 4 (same gate) hides `.camtoggle` entirely while saving, and none of the recording-only pieces (timer/paused-chip/Pause button) render either - just `[.grip][.exporting.saving][.spacer][.btn.rec, "Saving…"]`, 4 children / 3 gaps. `.btn.rec`'s own label is "Saving…" in this mode (wider than "Stop"), so it gets its own measured text-width term rather than reusing `RECORDING_WIDTH`'s.

### Used by

- `src/hud/hooks/useHudWindowSize.ts` - `hudLogicalSize` returns it while `mode === "saving"`.
- `src/hud/hooks/useHudWindowSize.test.ts` - asserts it stays narrower than both `WIDTH` and `RECORDING_WIDTH`.

## HudMode

```ts
export type HudMode = "idle" | "recording" | "saving"
```

The three bar states `hudLogicalSize`/`useHudWindowSize` size the window for. `Hud.tsx` derives this from its own `saving`/`recording` state (`saving` wins over `recording` - the two are mutually exclusive in practice, since `useRecordingFlow.finish` sets `recording` false and `saving` true together in the same render, before either commits).

### Used by

- `src/hud/hooks/useHudWindowSize.ts` - `hudLogicalSize`'s and `useHudWindowSize`'s own `mode` parameter.
- `src/hud/Hud.tsx` - computed inline at the `useHudWindowSize` call site.

## hudLogicalSize

```ts
export function hudLogicalSize(menu: string | null, mode: HudMode): { width: number; height: number }
```

Pure decision: the bar window's intended logical size for a given dropdown-menu + bar mode - `{ width: RECORDING_WIDTH, height: 132 }` while `mode === "recording"`, `{ width: SAVING_WIDTH, height: 132 }` while `mode === "saving"` (no dropdown can be open in either state - `Hud.tsx` only renders them while idle - so a stale `menu` value is ignored in both), otherwise `{ width: WIDTH, height: menu ? 430 : 132 }`. Extracted so the menu/mode-driven resize and the DPI-change re-normalization (both in `useHudWindowSize` below) always agree on the exact same numbers instead of each computing them separately, and so the decision is unit-testable without a Tauri window.

### Arguments

- `menu: string | null` - the currently-open dropdown id (or `null`), same as `useHudWindowSize`'s.
- `mode: HudMode` - the HUD's current bar state, same as `useHudWindowSize`'s.

### Used by

- `src/hud/hooks/useHudWindowSize.ts` - both `applySize` call sites (menu/mode changes and `onScaleChanged`).
- `src/hud/hooks/useHudWindowSize.test.ts` - regression coverage.

## useHudWindowSize

```ts
export function useHudWindowSize(menu: string | null, barShown: boolean, mode: HudMode): void
```

Side-effect-only hook; no return value.

### Arguments

- `menu: string | null` - `Hud`'s currently-open dropdown id (or `null`). *Why:* a taller window (430px vs. the idle 132px) is needed only while a dropdown's menu is open, so its list isn't clipped by the frame.
- `barShown: boolean` - whether the bar (as opposed to the Settings/Preferences box) is the visible content. *Why gated:* while a panel is open or mid-transition, `Hud.tsx`'s `openPanel`/`restoreBar` drive the window size themselves via `morphWindow`'s spring animation; this hook resizing at the same time would fight it.
- `mode: HudMode` - `"idle" | "recording" | "saving"`, computed by `Hud.tsx` from `useRecordingFlow`'s `recording`/`saving` flags (`saving` takes priority). *Why:* drives the content-fit resize - the window narrows to `RECORDING_WIDTH`/`SAVING_WIDTH` the instant `mode` leaves `"idle"`, and grows back to `WIDTH` the instant it returns.

### Behavior

**Sizing.** On every `menu` or `mode` change, while `barShown`, calls `win.setSize(new LogicalSize(...hudLogicalSize(menu, mode)))`.

**Cold-launch centering (L1).** `tauri.conf.json` boots the window at 860x132 with `center: true`, but this hook's very first resize (on mount) grows it to the real 980-wide bar. Tauri's `setSize` keeps the window's top-left corner fixed, so that growth alone would push the window 60px right of screen centre on every launch. A `centeredOnce` ref makes only the *first* resize also await completion and call `win.center()`. Later resizes (a dropdown opening/closing, or a mode change) deliberately do NOT re-center, so the bar never jumps out from under the cursor mid-interaction.

**DPI re-normalization.** Subscribes once (mount-only effect) to `win.onScaleChanged`, Tauri's signal that the window's scale factor moved (monitor swap, or the OS changing a display's scale under it). `setSize`'s `LogicalSize` -> physical-pixel conversion happens at call time using the scale factor *then*; if a DPI change lands right after (or the window was created before the OS settled on the new monitor's scale), the window's physical bounds stay pinned to the stale conversion while WebView2 renders CSS at the new factor, so the bar's `width:100%` no longer matches the window's real inner size and the right edge clips. The handler re-issues the same `setSize` (via a `latest` ref holding the current `menu`/`barShown`/`mode`, so it never acts on stale closure values) to re-derive the physical size from the *current* factor and *current mode* - it does not re-center, matching the dropdown-resize behavior above.

### Used by

- `src/hud/Hud.tsx` - called near the top of the component as `useHudWindowSize(menu, barShown, saving ? "saving" : recording ? "recording" : "idle")`.
