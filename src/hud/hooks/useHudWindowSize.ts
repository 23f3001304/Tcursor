import { useEffect, useRef } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

/** Fixed horizontal size of the HUD bar window while idle (only its height varies, with `menu`).
 *  Also the width `Hud.tsx`'s panel-morph transitions (`openPanel`/`restoreBar`) animate back out
 *  to once a recording (or a Settings/Preferences visit) ends. */
export const WIDTH = 980;

// RECORDING_WIDTH's terms, each traceable to a real hud.css value (or a measured text width where
// the CSS has none, e.g. a button's label) rather than one eyeballed constant - so the recording
// bar's window is sized to what it actually needs, not a guess (D5, user-reported 2026-09-02: the
// idle 980px window was kept for the ~470px-wide recording pill too, ballooning the invisible
// always-on-top hit-blocking slack around it). Sized for the WIDEST the recording row ever gets
// (paused, "Resume" not "Pause") so a pause/resume toggle never triggers a second mid-take resize.
const ROW_PAD = 12 * 2;                    // .row padding, both sides
const ROW_GAP = 6;                         // .row gap
const GRIP_W = 22;                         // .grip
const CAMTOGGLE_W = 52;                    // .camtoggle
// .recmeter padding + icon + gap + .wave (fixed, widest state). .wave itself is 13 bars * 3px +
// 12 gaps * 2px = 63px (gate-feedback item 2, user-reported 2026-09-02: the compact-meter redesign
// - see hud.css's `.wave`/`.wave span` and useMicWaveform.ts's `BARS = 13`), down from 80.
const RECMETER_W = 6 + 18 + 8 + 63 + 6;
const SPACER_MIN_W = 8;                    // .spacer min-width
const TIMER_W = 50;                        // .timer min-width
const PAUSED_CHIP_W = 62;                  // .paused-chip padding+border+"PAUSED" - measured
const PAUSE_BTN_W = 16 * 2 + 48;           // .btn padding + "Resume" (widest label) - measured
const REC_BTN_W = 18 * 2 + 8 + 7 + 34;     // .btn.rec padding + dot + gap + "Stop" - measured
const RECORDING_SAFETY = 12;               // slack for the two measured (not CSS-exact) text widths above

/** Horizontal size of the bar window while recording - see the block comment above for how each
 *  term was derived. Exported so `useHudWindowSize.test.ts` can assert it stays narrower than the
 *  idle `WIDTH` without hand-copying the arithmetic. */
export const RECORDING_WIDTH =
  ROW_PAD + GRIP_W + CAMTOGGLE_W + RECMETER_W + SPACER_MIN_W + TIMER_W +
  PAUSED_CHIP_W + PAUSE_BTN_W + REC_BTN_W + ROW_GAP * 7 + RECORDING_SAFETY;

// SAVING_WIDTH's terms - same methodology as RECORDING_WIDTH above (gate-feedback item 3,
// user-reported 2026-09-02: "Saving… X%" used to fall back to the idle 980px window around
// ~300px of actual content, since `recording` is already `false` by the time `saving` goes true -
// see useRecordingFlow.ts's `finish`). The saving row is SHORTER than the recording one: item 4
// (same gate) hides `.camtoggle` entirely while saving, and none of the recording-only pieces
// (timer/paused-chip/Pause button) render either - just [[.grip][.exporting.saving][.spacer]
// [.btn.rec, "Saving…"]], 4 children / 3 gaps.
const SAVING_TEXT_W = 8 + 78 + 8;          // .exporting padding (0 8px, both sides) + "Saving… 100%" (widest, monospace) - measured
const REC_BTN_SAVING_W = 18 * 2 + 8 + 7 + 55; // .btn.rec padding + dot + gap + "Saving…" (its own label here, wider than "Stop") - measured
const SAVING_SAFETY = 12;                  // slack for the two measured (not CSS-exact) text widths above

/** Horizontal size of the bar window while saving/preprocessing - see the block comment above.
 *  Exported so `useHudWindowSize.test.ts` can assert it too stays content-fit (narrower than the
 *  idle `WIDTH`), the same regression shape `RECORDING_WIDTH` already covers. */
export const SAVING_WIDTH =
  ROW_PAD + GRIP_W + SAVING_TEXT_W + SPACER_MIN_W + REC_BTN_SAVING_W + ROW_GAP * 3 + SAVING_SAFETY;

/** The three bar states `hudLogicalSize`/`useHudWindowSize` size the window for - `"idle"` (device
 *  pickers, width `WIDTH`), `"recording"` (content-fit `RECORDING_WIDTH`), and `"saving"`
 *  (content-fit `SAVING_WIDTH`, gate-feedback item 3). `Hud.tsx` derives this from its own
 *  `saving`/`recording` state (`saving` wins - the two are mutually exclusive in practice, since
 *  `useRecordingFlow.finish` sets `recording` false and `saving` true in the same render). */
export type HudMode = "idle" | "recording" | "saving";

/** Pure decision: the bar window's intended logical size for a given dropdown-menu + bar `mode`
 *  (taller while a menu needs the extra room; content-fit narrow while recording/saving, since no
 *  menu can ever be open in either of those - `Hud.tsx` only renders dropdowns while the bar is
 *  idle). Extracted so `useHudWindowSize` and its DPI-change re-normalization always apply the
 *  exact same numbers - and so the decision is unit-testable without a Tauri window. */
export function hudLogicalSize(menu: string | null, mode: HudMode): { width: number; height: number } {
  if (mode === "recording") return { width: RECORDING_WIDTH, height: 132 };
  if (mode === "saving") return { width: SAVING_WIDTH, height: 132 };
  return { width: WIDTH, height: menu ? 430 : 132 };
}

/** Keeps the HUD's OS window sized to the bar's current content (taller while a dropdown's menu
 *  needs the extra room, so it isn't clipped by the frame), and fixes the cold-launch off-centre
 *  bug (L1): `tauri.conf.json` boots the window at 860x132 with `center: true`, but this hook's
 *  very first resize grows it to the real 980-wide bar - `setSize` keeps the top-left corner
 *  fixed, so that growth otherwise pushes the window 60px right of screen centre on every launch.
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
 *  Only active while `barShown` - `Hud.tsx`'s panel-morph transitions own the window size
 *  themselves while a Settings/Preferences panel is open or animating.
 *
 *  `mode` (D5/item 3, user-reported 2026-09-02) drives the same seam: while `"recording"` or
 *  `"saving"` the window shrinks to that mode's own content-fit width and grows back to the idle
 *  `WIDTH` the instant `mode` goes back to `"idle"` - same `applySize` path as the menu resize
 *  above, so it gets the same one-time-centering and DPI re-normalization for free, and
 *  deliberately does NOT re-center either, so the window doesn't jump mid-take or mid-save. */
export function useHudWindowSize(menu: string | null, barShown: boolean, mode: HudMode) {
  const win = getCurrentWindow();
  const centeredOnce = useRef(false);
  // Always-current snapshot so the onScaleChanged listener (subscribed once, below) applies
  // whatever menu/barShown/mode are live at the moment the event fires, not whatever they were
  // when the listener was attached.
  const latest = useRef({ menu, barShown, mode });
  latest.current = { menu, barShown, mode };

  const applySize = (center: boolean) => {
    if (!latest.current.barShown) return;
    const { width, height } = hudLogicalSize(latest.current.menu, latest.current.mode);
    const resized = win.setSize(new LogicalSize(width, height));
    if (center) resized.then(() => win.center());
  };

  useEffect(() => {
    if (!barShown) return;
    applySize(!centeredOnce.current);
    centeredOnce.current = true;
  }, [menu, mode]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void win.onScaleChanged(() => applySize(false)).then((u) => {
      if (disposed) u();
      else unlisten = u;
    });
    return () => { disposed = true; unlisten?.(); };
  }, []);
}
