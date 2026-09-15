# src/hud/hooks/useHudWindowSize.ts

Owns the HUD's OS window sizing, for every state it has: keeps the window sized to the idle card, taller while a dropdown's menu is open, and content-fit around the take pill while recording or saving (D5, item 3), fixes the cold-launch off-centre bug (L1), and re-normalizes the window when the OS reports a DPI/scale-factor change so the bar's `width:100%` CSS (hud.css) never ends up wider than the window's actual physical bounds. Extracted out of `Hud.tsx` to keep that file under the project's 200-line cap - no behavior changed in the move. Since 2026-09-14 there is no `barShown` gate: Settings and Preferences open as sheets inside the card, at the card's own size, so nothing else ever drives the window.

## WIDTH

```ts
export const WIDTH = 360
```

Fixed horizontal size of the HUD window while idle: the vertical card (`IdleCard.tsx`, the owner's 2026-09-14 rethink of the 980px bar). Settings and Preferences are sheets inside that card, so opening one changes nothing here. Only the height varies, with whether a dropdown menu is open.

## CARD_HEIGHT

```ts
export const CARD_HEIGHT: number  // 510
```

Height of the card itself, border included, as a sum of named terms each a `hud.css` `.card*` value (padding 12 + 14, the 36px header, the 160px preview, three 46px rows, the 42px toggle row, the 48px Record, the gaps, the 2px border), so the window is sized to the card and not the card squeezed to a guessed window. The card's body measures 430 of that, which is what every other body state is pinned to (`--sheet-h`: the display picker's `.sheet` and the Settings/Preferences `.settings` box), so flipping the card never resizes the window.

## IDLE_HEIGHT

```ts
export const IDLE_HEIGHT: number  // 536
```

The window around the card: the body's 16px top padding (`hud.css`), `CARD_HEIGHT`, and 10px of air underneath. Not a shadow allowance any more: the card draws no outer shadow (`hud.css` `.hud`), because a transparent window cut this tight clipped the 40px blur into a hard-edged dim band under the card, the artifact the owner saw on the desktop (2026-09-15).

## MENU_OVERFLOW

```ts
export const MENU_OVERFLOW: number  // 140
```

How far the lowest open menu (the mic row's `.dd-menu` at its 256px max height plus its 6px offset) runs past the card's bottom edge. The window grows by this while any menu is open, so the list is never clipped by the frame. `Hud.tsx`'s `openPanel` clears `menu` on its way into a panel, so the window is never left tall for a list the panel has covered.

### Used by

- `src/hud/hooks/useHudWindowSize.ts` - `hudLogicalSize` builds on it.

## PillParts

```ts
export interface PillParts { cam: boolean; meter: boolean; warn: boolean; sources: boolean }
```

What the take pill is showing, which is what its window is sized for: the webcam circle only while the camera is on, the level slot only while an audio source is, the warning glyph only while there is a warning (`TakeBar`: a source that is off is not in the pill at all, owner 2026-09-14).

`sources` is the odd one out, and deliberately so. The Sources sheet (`SourcesSheet.tsx`, mid-take source switching) hangs UNDER the pill rather than inside it, so it changes the window's HEIGHT only: `takeWidth` never reads the field, and the pill neither moves nor resizes when the sheet opens. It lives here rather than as a fourth argument because it is another fact about what the take surface is currently showing, and `Hud` already threads exactly one such object through.

## FULL_PILL

```ts
export const FULL_PILL: PillParts  // { cam: true, meter: true, warn: false, sources: false }
```

The widest ordinary pill: everything on. What `TAKE_WIDTH` measures and what `hudLogicalSize`/`useHudWindowSize` assume when no `pill` is passed.

## takeWidth

```ts
export function takeWidth(parts: PillParts): number
```

Horizontal size of the bar window around a pill showing `parts`: the always-present clock and three buttons (Sources, Pause, Stop - four children, three gaps) plus each optional child with the gap it brings (the 44px circle, the 4 + 150px slot, the 20px warning glyph). `parts.sources` is deliberately NOT read: the sheet is under the pill, not in it. Tests: `the pill's window is sized for what it shows`, `the Sources sheet adds its own height and nothing else`.

## TAKE_WIDTH

```ts
export const TAKE_WIDTH: number  // 473, takeWidth(FULL_PILL)
```

Horizontal size of the bar window while recording or saving: the take pill (`TakeBar.tsx`, the 2026-09-14 rethink of the recording state). Computed, not eyeballed: a sum of named constants each traceable to a real `hud.css` `.take*` value (`.take`'s padding and gap, `.camtoggle.round`, `.take-clock`'s dot and timer `min-width`, `.take-btn` three times - Sources, Pause and Stop) or to the component that owns it (`SLOT_W` takes `RecMeter.tsx`'s exported `METER_W` **by import**, not by a copied number, so the window and the slot can never disagree about its width), plus a small safety margin for the `.hud` border and a timer past 9:59. The pill is the SAME width in every take state - Paused and the struck mic swap into the slot's fixed box, and the saving pill's progress line stretches to fill - so a take never resizes the window between Record and the editor. It replaced `RECORDING_WIDTH` (579) and `SAVING_WIDTH` (284), which sized the old recording row and saving row separately.

### Used by

- `src/hud/hooks/useHudWindowSize.ts` - `hudLogicalSize` returns it for both take modes.
- `src/hud/hooks/useHudWindowSize.test.ts` - pins the number (`the everything-on pill is 473 wide`), so it cannot drift from what this doc quotes.
- `src/hud/Hud.tsx` - pins the `.hud` surface to this width by inline style while the pill is the shown state, so the pill holds its shape while the window is still gliding down to it.

## TAKE_HEIGHT

```ts
export const TAKE_HEIGHT: number  // 88
```

Vertical size of the window around the pill: the body's 16px top padding (`hud.css`), the 60px pill with its 1px border top and bottom, and 10px of air underneath (no shadow, see `IDLE_HEIGHT`). The idle bar's 132 stays as the literal it always was.

### Used by

- `src/hud/hooks/useHudWindowSize.ts` - `hudLogicalSize` returns it (plus the sheet, when open) for both take modes.
- `src/hud/hooks/useHudWindowSize.test.ts` - asserts it is under the idle 132.

## SOURCES_HEIGHT

```ts
export const SOURCES_HEIGHT: number  // 168
```

How much taller the take window is while the Sources sheet is open. Derived the same way `IDLE_HEIGHT` is derived from the card: three 46px `.dd-row`s (display, camera, microphone) with the two 8px gaps between them and the sheet's own 4 + 10px padding, each a real `hud.css` `.src-sheet` value. Fixed, so flipping the sheet to the display list never resizes the window a second time - the list scrolls inside the same box, exactly as `.sheet` does in the idle card.

## SOURCES_MENU_OVERFLOW

```ts
export const SOURCES_MENU_OVERFLOW: number  // 262
```

How much taller again while a dropdown opened FROM the Sources sheet is showing. Larger than the idle card's `MENU_OVERFLOW` (140) because the sheet's last row sits a few px above the window's bottom edge, so a `.dd-menu` has nothing below it to fall into: the window has to grow by the menu's whole 256px max height plus its 6px offset. In the idle card two of the three rows have card underneath them, which is why 140 is enough there.

## HudMode

```ts
export type HudMode = "idle" | "recording" | "saving"
```

The three bar states `hudLogicalSize`/`useHudWindowSize` size the window for: `"idle"` (the device-picker bar) and the two take-pill states, which share one size. `Hud.tsx` derives this from its own `saving`/`recording` state (`saving` wins over `recording` - the two are mutually exclusive in practice, since `useRecordingFlow.finish` sets `recording` false and `saving` true together in the same render, before either commits).

### Used by

- `src/hud/hooks/useHudWindowSize.ts` - `hudLogicalSize`'s and `useHudWindowSize`'s own `mode` parameter.
- `src/hud/Hud.tsx` - computed inline at the `useHudWindowSize` call site.

## hudLogicalSize

```ts
export function hudLogicalSize(menu: string | null, mode: HudMode, pill?: PillParts): { width: number; height: number }
```

Pure decision: the HUD window's intended logical size for a given dropdown-menu + bar mode - `{ width: takeWidth(pill), height: TAKE_HEIGHT }` for either take mode (`pill` defaults to `FULL_PILL`), plus `SOURCES_HEIGHT` while `pill.sources`, plus `SOURCES_MENU_OVERFLOW` on top of that while a `menu` is also open. A `menu` only counts while the sheet is open: that sheet is the only thing in a take that can open one, so a `menu` left over from before Record never grows the bare pill. Otherwise `{ width: WIDTH, height: menu ? IDLE_HEIGHT + MENU_OVERFLOW : IDLE_HEIGHT }`. Extracted so the menu/mode-driven resize and the DPI-change re-normalization (both in `useHudWindowSize` below) always agree on the exact same numbers instead of each computing them separately, and so the decision is unit-testable without a Tauri window.

### Arguments

- `menu: string | null` - the currently-open dropdown id (or `null`), same as `useHudWindowSize`'s.
- `mode: HudMode` - the HUD's current bar state, same as `useHudWindowSize`'s.

### Used by

- `src/hud/hooks/useHudWindowSize.ts` - both `applySize` call sites (menu/mode changes and `onScaleChanged`).
- `src/hud/hooks/useHudWindowSize.test.ts` - regression coverage.

## useHudWindowSize

```ts
export function useHudWindowSize(menu: string | null, mode: HudMode, pill?: PillParts): void
```

Side-effect-only hook; no return value.

After a snap resize (`setSize`, and `center()` on the cold launch) `applySize` awaits `keepHidden()` from `morph.ts`, re-asserting the HUD's exclude-from-capture flag; the glide path gets the same from inside `morphWindow`. See `keepHidden` for why.

### Arguments

- `menu: string | null` - `Hud`'s currently-open dropdown id (or `null`). *Why:* a taller window (`MENU_OVERFLOW` more) is needed only while a dropdown's menu is open, so its list isn't clipped by the frame.
- `mode: HudMode` - `"idle" | "recording" | "saving"`, computed by `Hud.tsx` from `useRecordingFlow`'s `recording`/`saving` flags (`saving` takes priority). *Why:* drives the content-fit resize - the window glides down to the pill (`TAKE_WIDTH` by `TAKE_HEIGHT`) when `mode` leaves `"idle"`, and back up to `WIDTH` by `IDLE_HEIGHT` when it returns. `Hud.tsx` passes the mode of the state that is SHOWN (`takeShown`), not the flow's own, so the glide waits for the leaving state to frost out first.

- `pill?: PillParts` - what the take surface is showing (`Hud.tsx`: `{ cam: camOn, meter: micOn || sysOn, warn: warn != null, sources: sourcesOpen }`); `FULL_PILL` when omitted. *Why:* the pill hides a source that is off, so its window is sized per take; a change in any part - `sources` included - while a take shows re-applies the size.

### Behavior

**Sizing.** On every `menu`, `mode` or `pill` change, calls `win.setSize(new LogicalSize(...hudLogicalSize(menu, mode, pill)))`. There is no gate: a Settings or Preferences panel is a body state of the idle card, so the idle size covers it.

**Gliding between the bar and the pill.** `setSize` keeps the window's top-left corner, so snapping the 360px card to the 473px pill would leave the pill hanging off the bar's left end, nowhere near where the eye (or the cursor, on Record) was. When the mode differs from the one last sized (`sizedMode` ref), `applySize` calls `morphWindow(from, to, 220, "centre")` instead: the window glides between `hudLogicalSize` of the previous mode and of the new one over 220ms, moved each frame so its horizontal centre never shifts. Kept, not re-centred: the bar stays wherever the user put it. `Hud` hands this hook a `mode` that lags the flow by one interstate (`takeShown`, set by `StateSwap`'s `onSettled`), so the glide starts as the pill begins to arrive and the two land together. The Sources sheet opening or closing glides on the same path (a second ref, `sizedSheet`, tracks it): it adds 168px under a pill that is itself standing still, and a jump that size under a frosting-in sheet reads as a glitch. Menu-driven resizes and DPI re-normalization still snap and do not move the window.

**Cold-launch centering (L1).** `tauri.conf.json` boots the window at 860x132 with `center: true`, but this hook's very first resize (on mount) turns it into the 360-wide card. Tauri's `setSize` keeps the window's top-left corner fixed, so that change alone would leave the window off screen centre on every launch. A `centeredOnce` ref makes only the *first* resize also await completion and call `win.center()`. Later resizes (a dropdown opening/closing) deliberately do NOT re-center, so the bar never jumps out from under the cursor mid-interaction; a mode change keeps the window's own centre instead (above).

**DPI re-normalization.** Subscribes once (mount-only effect) to `win.onScaleChanged`, Tauri's signal that the window's scale factor moved (monitor swap, or the OS changing a display's scale under it). `setSize`'s `LogicalSize` -> physical-pixel conversion happens at call time using the scale factor *then*; if a DPI change lands right after (or the window was created before the OS settled on the new monitor's scale), the window's physical bounds stay pinned to the stale conversion while WebView2 renders CSS at the new factor, so the bar's `width:100%` no longer matches the window's real inner size and the right edge clips. The handler re-issues the same `setSize` (via a `latest` ref holding the current `menu`/`mode`/`pill`, so it never acts on stale closure values) to re-derive the physical size from the *current* factor and *current mode* - it does not re-center, matching the dropdown-resize behavior above.

### Used by

- `src/hud/Hud.tsx` - called near the top of the component as `useHudWindowSize(menu, takeShown ? (saving ? "saving" : "recording") : "idle", pill)`.
