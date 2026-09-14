import { useEffect, useRef } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { METER_W } from "../components/RecMeter";
import { keepHidden, morphWindow } from "../components/morph";

/** Fixed horizontal size of the HUD window while idle: the vertical card (`IdleCard.tsx`, owner's
 *  2026-09-14 rethink of the 980px bar). Settings and Preferences are sheets inside that card, so
 *  opening one changes nothing here. Only the height varies, with `menu`. */
export const WIDTH = 360;

// IDLE_HEIGHT's terms, each a hud.css `.card*` value, so the window is sized to the card and not
// the card squeezed to a guessed window.
const CARD_PAD = 12 + 14;                  // .card padding, top and bottom
const HEAD_H = 36;                         // .card-head
const PREVIEW_H = 160;                     // .camtoggle.wide
const ROW_H = 46;                          // .dd-row, three of them
const TOG_H = 42;                          // .tog-row
const REC_H = 48;                          // .btn.rec in the card
const CARD_GAPS = 8 * 6 + 10;              // .card gap under the head, the five .card-body gaps, the extra before Record

/** Height of the card itself, border included; also what every other body state (`.sheet`, and the
 *  Settings/Preferences panel) is pinned to, so flipping the card never resizes the window. */
export const CARD_HEIGHT = CARD_PAD + HEAD_H + PREVIEW_H + ROW_H * 3 + TOG_H + REC_H + CARD_GAPS + 2;
/** The window around it: the body's 16px top padding and 10px for the shadow. */
export const IDLE_HEIGHT = 16 + CARD_HEIGHT + 10;
/** How far the lowest open menu (the mic row's, `.dd-menu` at its 256px max height plus its 6px
 *  offset) runs past the card's bottom edge; the window grows by this while any menu is open. */
export const MENU_OVERFLOW = 140;

// TAKE_WIDTH's terms, each traceable to a real hud.css `.take*` value (or `RecMeter.tsx`'s own
// `METER_W`, by import) rather than one eyeballed constant, so the take pill's window is sized to
// what it actually needs (D5, user-reported 2026-09-02: the idle 980px window was kept for the
// much narrower recording content, ballooning the invisible always-on-top hit-blocking slack).
// The pill is the same width in EVERY take state - Paused and the struck mic swap into the level
// slot's fixed box, and the saving pill's progress line stretches to fill - so a take never
// resizes the window between Record and the editor.
const TAKE_PAD = 8 * 2;                    // .take padding, both sides
const TAKE_GAP = 10;                       // .take gap, one between each pair of children
const CAM_ROUND_W = 44;                    // .camtoggle.round
const CLOCK_W = 6 + 9 + 6 + 52;            // .take-clock margin + dot + gap + timer min-width
const SLOT_W = 4 + METER_W;                // .take-slot margin + the meter's own width
const TAKE_BTN_W = 42;                     // .take-btn, Sources, Pause and Stop
const TAKE_SAFETY = 10;                    // the .hud border and slack for a timer past 9:59

/** What the take pill is showing, which is what its window is sized for: the webcam circle
 *  only while the camera is on, the level slot only while an audio source is, the warning glyph
 *  only while there is a warning (`TakeBar`: a source that is off is not in the pill at all).
 *  `sources` is the odd one out: the Sources sheet hangs UNDER the pill, so it changes the
 *  window's HEIGHT only - `takeWidth` deliberately never reads it, and the pill neither moves nor
 *  resizes when the sheet opens. */
export interface PillParts { cam: boolean; meter: boolean; warn: boolean; sources: boolean }
const WARN_W = 20;                         // .take-warn
/** The widest pill: everything shown. What `TAKE_WIDTH` measures. */
export const FULL_PILL: PillParts = { cam: true, meter: true, warn: false, sources: false };

/** Horizontal size of the bar window around a pill showing `parts` - the always-present clock and
 *  three buttons (four children, three gaps), plus each optional child with the gap it brings. */
export function takeWidth(parts: PillParts): number {
  return TAKE_PAD + CLOCK_W + TAKE_BTN_W * 3 + TAKE_GAP * 3 + TAKE_SAFETY
    + (parts.cam ? CAM_ROUND_W + TAKE_GAP : 0) + (parts.meter ? SLOT_W + TAKE_GAP : 0) + (parts.warn ? WARN_W + TAKE_GAP : 0);
}

/** `takeWidth(FULL_PILL)` - see the block comment above for how each term was derived. Exported so
 *  `useHudWindowSize.test.ts` can assert it stays narrower than the idle `WIDTH` without
 *  hand-copying the arithmetic. */
export const TAKE_WIDTH = takeWidth(FULL_PILL);

/** Vertical size of the window around the 60px pill: the body's 16px top padding (hud.css), the
 *  pill with its 1px border top and bottom, and 10px for the shadow to fall into. */
export const TAKE_HEIGHT = 16 + 62 + 10;

// SOURCES_HEIGHT's terms, each a hud.css `.src-sheet` value, the same way IDLE_HEIGHT is derived
// from the card: three 46px `.dd-row`s (display, camera, microphone) with the sheet's own padding
// and the two gaps between them. Fixed, so flipping the sheet to the display list never resizes
// the window a second time (the list scrolls inside the same box, exactly as `.sheet` does in the
// card).
const SRC_ROWS = 46 * 3, SRC_GAPS = 8 * 2, SRC_PAD = 4 + 10;
/** How much taller the take window is while the Sources sheet is open. */
export const SOURCES_HEIGHT = SRC_ROWS + SRC_GAPS + SRC_PAD;
/** A menu opened from the Sources sheet hangs off its LAST row, a few px above the window's
 *  bottom edge, so the window must grow by the whole `.dd-menu` (256px max height plus its 6px
 *  offset) - unlike the idle card, where two of the three rows have card below them to fall into. */
export const SOURCES_MENU_OVERFLOW = 262;

/** The three bar states `hudLogicalSize`/`useHudWindowSize` size the window for - `"idle"` (device
 *  card, width `WIDTH`) and the two take-pill states `"recording"` and `"saving"` (both the
 *  content-fit `TAKE_WIDTH` by `TAKE_HEIGHT`, gate-feedback item 3). `Hud.tsx` derives this from its own
 *  `saving`/`recording` state (`saving` wins - the two are mutually exclusive in practice, since
 *  `useRecordingFlow.finish` sets `recording` false and `saving` true in the same render). */
export type HudMode = "idle" | "recording" | "saving";

/** Pure decision: the HUD window's intended logical size for a given dropdown-menu + bar `mode`
 *  (taller while a menu needs the extra room; the pill's own size while recording/saving, since no
 *  menu can ever be open in either of those - `Hud.tsx` only renders dropdowns while idle). Extracted so `useHudWindowSize` and its DPI-change re-normalization always apply the
 *  exact same numbers - and so the decision is unit-testable without a Tauri window. */
export function hudLogicalSize(menu: string | null, mode: HudMode, pill: PillParts = FULL_PILL): { width: number; height: number } {
  if (mode !== "idle") {
    // A menu only counts while the Sources sheet is open - that is the only thing in a take that
    // can open one, so a `menu` left over from before Record must not grow the bare pill.
    const sheet = pill.sources ? SOURCES_HEIGHT + (menu ? SOURCES_MENU_OVERFLOW : 0) : 0;
    return { width: takeWidth(pill), height: TAKE_HEIGHT + sheet };
  }
  return { width: WIDTH, height: menu ? IDLE_HEIGHT + MENU_OVERFLOW : IDLE_HEIGHT };
}

/** Keeps the HUD's OS window sized to the bar's current content (taller while a dropdown's menu
 *  needs the extra room, so it isn't clipped by the frame), and fixes the cold-launch off-centre
 *  bug (L1): `tauri.conf.json` boots the window at 860x132 with `center: true`, but this hook's
 *  very first resize turns it into the 360-wide card - `setSize` keeps the top-left corner
 *  fixed, so that change otherwise leaves the window off screen centre on every launch.
 *  Re-centers exactly once, right after that first resize settles; later resizes (a dropdown
 *  opening/closing) deliberately do NOT re-center, so the bar never jumps out from under the
 *  cursor mid-interaction.
 *
 *  Also re-applies the same logical size whenever Tauri reports the window's scale factor changed
 *  (moving it to a different-DPI monitor, or Windows changing a display's scale under it). Tauri
 *  converts `LogicalSize` -> physical pixels using the scale factor at the moment `setSize` is
 *  called; if that call raced a DPI change (or the window just sat on a monitor whose scale
 *  changed under it), the window's physical bounds stay pinned to the stale factor's math while
 *  WebView2 keeps rendering CSS at the new one, so 980 logical px of bar content no longer fits
 *  the physical window and the right edge clips. `onScaleChanged` is Tauri's own signal that the
 *  factor moved (`WINDOW_SCALE_FACTOR_CHANGED`) - re-issuing the same `setSize` re-derives the
 *  physical size from the *current* factor and the window snaps back to matching its CSS content
 *  (hud.css's `.hud` also lays out to `width:100%` of that window as defensive backup, not as the
 *  primary fix - this re-normalization is what keeps the two from drifting apart in the first
 *  place). Does not re-center, matching the dropdown-resize behavior above. Reads `mode` from the
 *  `latest` ref (below) at fire time, so it re-normalizes to whatever mode is CURRENTLY live, not
 *  whatever it was when the listener was attached.
 *
 *  Always active: Settings and Preferences are sheets inside the idle card now (owner 2026-09-14),
 *  so nothing else ever owns the window's size and there is no `barShown` gate left to fight over.
 *
 *  `mode` (D5/item 3, user-reported 2026-09-02) drives the same seam: while `"recording"` or
 *  `"saving"` the window shrinks to the pill's content-fit size and grows back to the idle
 *  `WIDTH` the instant `mode` goes back to `"idle"` - same `applySize` path as the menu resize
 *  above, so it gets the same one-time-centering and DPI re-normalization for free, and
 *  deliberately does NOT re-center either, so the window doesn't jump mid-take or mid-save - it
 *  glides between the two sizes about its own horizontal centre instead (see `applySize`). */
export function useHudWindowSize(menu: string | null, mode: HudMode, pill: PillParts = FULL_PILL) {
  const win = getCurrentWindow();
  const centeredOnce = useRef(false);
  // Always-current snapshot so the onScaleChanged listener (subscribed once, below) applies
  // whatever menu/mode are live at the moment the event fires, not whatever they were when the
  // listener was attached.
  const latest = useRef({ menu, mode, pill });
  latest.current = { menu, mode, pill };

  const sizedMode = useRef(mode);
  // The Sources sheet opening or closing is the other size change that must GLIDE rather than
  // snap: it adds `SOURCES_HEIGHT` under a pill that is itself standing still, and a 168px jump
  // under a frosting-in sheet reads as a glitch.
  const sizedSheet = useRef(pill.sources);

  const applySize = async (center: boolean) => {
    const { width, height } = hudLogicalSize(latest.current.menu, latest.current.mode, latest.current.pill);
    // A mode change swaps the idle bar for the take pill (or back): the window GLIDES between the
    // two sizes (220ms) about its horizontal centre, so the pill lands where the bar was instead
    // of hanging off its left end - kept, not re-centred on the screen: the bar stays wherever the
    // user put it. Menu and DPI resizes still snap.
    const was = { ...latest.current.pill, sources: sizedSheet.current };
    const from = hudLogicalSize(latest.current.menu, sizedMode.current, was);
    const glide = !center
      && (latest.current.mode !== sizedMode.current || latest.current.pill.sources !== sizedSheet.current);
    sizedMode.current = latest.current.mode;
    sizedSheet.current = latest.current.pill.sources;
    if (glide) { await morphWindow(from.width, from.height, width, height, 220, "centre"); return; }
    await win.setSize(new LogicalSize(width, height));
    if (center) await win.center();
    await keepHidden(); // the glide path re-asserts it inside `morphWindow`
  };

  useEffect(() => {
    void applySize(!centeredOnce.current);
    centeredOnce.current = true;
  }, [menu, mode, pill.cam, pill.meter, pill.warn, pill.sources]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void win.onScaleChanged(() => void applySize(false)).then((u) => {
      if (disposed) u();
      else unlisten = u;
    });
    return () => { disposed = true; unlisten?.(); };
  }, []);
}
