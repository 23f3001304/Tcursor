# src/editor/stage/Transport.tsx

The transport bar rendered between the Stage and the Timeline. Left: Trim In/Out buttons that cut the clip straight to the playhead, a Reset button that appears only once trimmed, then a divider plus add-zoom / AI-autoedit tools. Center: skip-to-start / play-pause / skip-to-end and the time readout. Right: an aspect-ratio chip, a preview-quality chip, and mute + a real volume flyout. Local state is limited to the volume flyout's open/closed flag (`showVolumeSlider`) - `volume` itself is now a prop owned by `Editor` (the same value `Stage` applies as real audio gain), not a local mock.

## Transport

```tsx
export const Transport: React.MemoExoticComponent<(props: {
  timeMs: number; dur: number; playing: boolean; onPlay: () => void; onSeek: (ms: number) => void;
  onAddZoom: () => void; onAutoedit: () => void; aiRunning: boolean; exporting: boolean;
  trimmed: boolean; onTrimIn: () => void; onTrimOut: () => void; onResetTrim: () => void;
  aspect: Aspect; onAspect: (aspect: Aspect) => void;
  quality: number; onQuality: () => void;
  muted: boolean; onMute: () => void;
  volume: number; onVolume: (v: number) => void;
}) => JSX.Element>
```

Renders the transport bar. `React.memo`'d (render hygiene pass) - `timeMs` still ticks every frame during playback (the time readout genuinely needs it live), so this can't skip re-rendering entirely, but memo still avoids a re-render from unrelated `Editor` state as long as the caller passes stable callback props (`Editor.tsx`'s `useEditorCallbacks`).

### Props

- `timeMs: number` / `dur: number` - the playhead and total clip duration (ms); shown as `fmt(timeMs) / fmt(dur)`.
- `playing: boolean` / `onPlay: () => void` - play/pause state and toggle. *Behavior owned by the caller:* `Editor.tsx`'s `onPlay` snaps the playhead forward to the trim-in point when starting playback from outside the trim range (see `resolveTrim`).
- `onSeek: (ms: number) => void` - called with `0` (skip-to-start) or `dur` (skip-to-end).
- `onAddZoom: () => void` - adds a zoom at the playhead.
- `onAutoedit: () => void` - runs the AI auto-director.
- `aiRunning: boolean` - whether an AI director run is in flight (`Editor`'s `running` state); disables the wand button so a double click (or the wand plus a keyboard trigger) can't start a second interleaved reveal. `Editor`'s `onRun` also re-entrancy-guards itself as a backstop.
- `exporting: boolean` (Task 36) - whether an export pipeline is running (`Editor`'s lifted `exporting` state, the same one `TopBar` reads). Combined with `dur <= 0` into a local `locked` flag that disables Play, Trim In/Out/Reset, and the aspect chip - none of them should change what's rendering mid-export, and none are meaningful with no clip loaded yet. Also now disables the AI-director wand directly (bug-sweep-2 Task 8, L3 - see the wand's own note below): previously only `aiRunning` gated it, so nothing actually stopped a director run from starting mid-export.
- `trimmed: boolean` - whether the doc's trim range is currently narrower than the whole clip (`resolveTrim(doc.trim, dur)` != `[0, dur]`). Drives whether the Reset button renders at all.
- `onTrimIn: () => void` / `onTrimOut: () => void` - trim the clip's start/end straight to the current playhead (`useTrimActions` in `Editor.tsx`, which emits `set_trim` ops and guards each direction from collapsing the range - In ignores a click at/past the current out, Out ignores a click at/before the current in).
- `onResetTrim: () => void` - clears the trim back to the whole clip (`SetTrim { in_ms: 0, out_ms: 0 }`). The Reset button only renders when `trimmed` is true (nothing to reset otherwise) - dragging `TrimOverlay`'s edge handles on the timeline is still an alternate way to set the same trim range.
- `aspect: Aspect` / `onAspect: (aspect: Aspect) => void` - the doc's output aspect ratio and its setter (`SetAspect`). The chip cycles through all 5 `Aspect` values in `ASPECT_ORDER` on click, labelled via `ASPECT_LABEL`.
- `quality: number` / `onQuality: () => void` - the preview proxy height (480/720/1080) and its cycle handler (unrelated to `aspect`: this is the proxy-video transcode resolution, not the compositing canvas size).
- `muted: boolean` / `onMute: () => void` - audio mute toggle.
- `volume: number` / `onVolume: (v: number) => void` - the preview-audio volume as `0..100`, owned by `Editor`. The same value (divided by 100) is what `Stage` actually applies as the hidden `<audio>` element's gain via `useMediaPlayback` - it has to live above both components rather than as local state here.

### Behavior

**Locked state (Task 36).** `locked = exporting || dur <= 0`, computed once per render. Play, both trim buttons, the reset button, and the aspect chip all pass `disabled={locked}` (styled via `.e-play:disabled`/`.e-tbtn:disabled`/`.e-tg:disabled`/`.e-chip:disabled` - dimmed, `cursor: default`). Play and the reset button are `motion.button`s whose `whileTap` spring is also omitted while `locked` (spread conditionally, `locked ? undefined : ...`) so a disabled button doesn't visually invite a press it will ignore; the trim buttons and the aspect chip have no Motion at all - their own hover-lift + `:active` press is plain CSS (design/premium-pass D6 leaves `.e-tbtn`/`.e-chip` alone, see `editor.css`). Skip-to-start/end, the quality chip, and mute are unaffected by locking - they're safe (or still meaningful) during either state.

**Aspect chip.** `cycleAspect` finds `aspect`'s index in `ASPECT_ORDER` and calls `onAspect` with the next one (wrapping); the chip label comes from `ASPECT_LABEL[aspect]` (`"Source"`, `"16:9"`, `"9:16"`, `"1:1"`, `"4:3"`). Both `ASPECT_ORDER` and `ASPECT_LABEL` are `export`ed (Task 11) so `StageToolbar`'s own aspect quick-toggle cycles the identical sequence rather than a second, possibly-diverging copy - see `StageToolbar.md`.

**Trim In / Out.** Two `.e-tbtn` buttons (`IconArrowBarToLeft`/`IconArrowBarToRight`, labelled "In"/"Out"), `disabled={locked}`, call `onTrimIn`/`onTrimOut` to cut the clip's start/end to wherever the playhead currently sits. A third button (`motion.button.e-tg on`, `IconX`, title "Reset the trim range"), also `disabled={locked}`, renders only when `trimmed` and calls `onResetTrim` - there's no separate disabled "pill" state for its own absence, the reset control simply isn't in the DOM until there's something to reset. Its press (`whileTap: { scale: 0.96 }` when not `locked`, `PLAY_SPRING`'s stiffness/damping) is the app-wide press spring (design/premium-pass D6) filling a gap the plain `.e-tbtn`/`.e-tg` buttons around it didn't have.

**Add zoom + AI Auto-director wand.** Both are `motion.button.e-tg` sharing a tap/hover spring (`TAP_SPRING`: `whileHover: scale 1.04`, `whileTap: scale 0.98`, `0.12s` tween, Task 36). Add-zoom (`IconZoomIn`, title "Add a zoom region here (Z)") calls `onAddZoom` unconditionally. The wand (`IconWand`, title "Run the AI director to auto-edit this clip") calls `onAutoedit`, `disabled={aiRunning || exporting}` (bug-sweep-2 Task 8, L3 added the `exporting` half - see `exporting`'s own note above) - grayed out (`.e-tg:disabled`, `opacity: .4`) and inert (spring omitted too) for the whole director run OR export, not just until the click registers. Carries `data-director-anchor="wand"` - `src/editor/director/targets.ts`'s `anchorPoint("wand")` uses this (alongside the AiPanel run button, which carries the same attribute) to find where the AI director's fake pointer should start a run from.

**Playback controls (design/premium-pass D3: Play restyled as the bar's hero).** Skip-to-start (title "Jump to the start") calls `onSeek(0)`; skip-to-start/end are now `motion.button`s carrying the app-wide press spring (design/premium-pass D6: `whileTap: { scale: 0.96 }`, unconditional - neither is gated by `locked`) since they previously had no press feedback at all. Play/Pause (`.e-play`, `disabled={locked}`) calls `onPlay()` and shows `IconPlayerPause`/`IconPlayerPlay` by `playing` (both `fill="currentColor"` so they render as solid glyphs rather than outlines, and the play triangle carries a `marginLeft: 2` nudge to sit visually centered). Play gets its own spring (`PLAY_SPRING`: `type: "spring", stiffness: 500, damping: 30`, the same spring `TopBar`'s Export button now uses) rather than `TAP_SPRING`, with `whileTap: { scale: 0.94 }` (omitted while `locked`) - snappier than the ghost buttons around it. A `.e-play-glow` overlay (the existing `--e-glow-primary` soft-glow token) fires a one-shot 150ms fade-out pulse on every play/pause toggle: its `key` alternates between `"on"`/`"off"` with `playing`, so React remounts it (replaying `initial={{opacity:1}} -> animate={{opacity:0}}`) each time `playing` flips, with no separate pulse-tracking state needed. Skip-to-end (title "Jump to the end") calls `onSeek(dur)`.

**Time readout.** The current position renders via `fmtPrecise` (`src/editor/timeline/time.ts`, "M:SS.s" - one decisecond) in `.e-time`'s own 13px/600 `--e-fg`; the dimmed `.tot` total still uses plain `fmt` ("M:SS", no decimal) so only the live position reads to a tenth of a second.

**Mute button.** Shows `IconVolumeOff` when `muted` is true OR `volume === 0` (so a slider dragged all the way down reads as muted even before the mute button itself is clicked), otherwise `IconVolume`. The click handler is always `onMute` - a plain toggle that doesn't touch `volume`. Also a `motion.button` with the app-wide press spring (design/premium-pass D6), unconditional since mute is never locked.

**Volume flyout.** Hovering `.e-volwrap` reveals (`AnimatePresence` fade+slide, no width animation) a `Slider` bound directly to the `volume`/`onVolume` props (`0..100`, displaying `0` while `muted` without altering the underlying `volume` value) - a real, `Editor`-owned control now rather than a local mock. Dragging it above `0` while `muted` also calls `onMute()`, so raising the slider from a muted state unmutes rather than silently changing a number nobody hears. **Hover bridge (Task 36):** `.e-volwrap` has no flex `gap` between the mute button and `.e-volflyout` - the same 6px of breathing room instead lives as `.e-volflyout`'s own `padding-left`, so the button and flyout sit flush with zero dead space between them and the wrap's hover state (which shows/hides the flyout) never has a gap to lose the pointer over while crossing from the button onto the slider.

### Notes

- Transport owns no edit-doc state directly - `trimmed`/`aspect`/`quality`/`muted`/`volume`/`exporting` are all derived/owned by `Editor.tsx` and passed down; only the volume flyout's open/closed flag (`showVolumeSlider`) is local. `volume` used to be a local mock but is real, lifted state as of this change.
