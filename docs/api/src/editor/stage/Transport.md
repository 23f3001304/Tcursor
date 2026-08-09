# src/editor/stage/Transport.tsx

The transport bar rendered between the Stage and the Timeline. Left: Trim In/Out buttons that cut the clip straight to the playhead, a Reset button that appears only once trimmed, then a divider plus add-zoom / AI-autoedit tools. Center: skip-to-start / play-pause / skip-to-end and the time readout. Right: an aspect-ratio chip, a preview-quality chip, and mute + a real volume flyout. Local state is limited to the volume flyout's open/closed flag (`showVolumeSlider`) - `volume` itself is now a prop owned by `Editor` (the same value `Stage` applies as real audio gain), not a local mock.

## Transport

```tsx
export function Transport({
  timeMs, dur, playing, onPlay, onSeek, onAddZoom, onAutoedit, aiRunning, exporting,
  trimmed, onTrimIn, onTrimOut, onResetTrim, aspect, onAspect, quality, onQuality, muted, onMute,
  volume, onVolume,
}: {
  timeMs: number; dur: number; playing: boolean; onPlay: () => void; onSeek: (ms: number) => void;
  onAddZoom: () => void; onAutoedit: () => void; aiRunning: boolean; exporting: boolean;
  trimmed: boolean; onTrimIn: () => void; onTrimOut: () => void; onResetTrim: () => void;
  aspect: Aspect; onAspect: (aspect: Aspect) => void;
  quality: number; onQuality: () => void;
  muted: boolean; onMute: () => void;
  volume: number; onVolume: (v: number) => void;
}): JSX.Element
```

Renders the transport bar.

### Props

- `timeMs: number` / `dur: number` - the playhead and total clip duration (ms); shown as `fmt(timeMs) / fmt(dur)`.
- `playing: boolean` / `onPlay: () => void` - play/pause state and toggle. *Behavior owned by the caller:* `Editor.tsx`'s `onPlay` snaps the playhead forward to the trim-in point when starting playback from outside the trim range (see `resolveTrim`).
- `onSeek: (ms: number) => void` - called with `0` (skip-to-start) or `dur` (skip-to-end).
- `onAddZoom: () => void` - adds a zoom at the playhead.
- `onAutoedit: () => void` - runs the AI auto-director.
- `aiRunning: boolean` - whether an AI director run is in flight (`Editor`'s `running` state); disables the wand button so a double click (or the wand plus a keyboard trigger) can't start a second interleaved reveal. `Editor`'s `onRun` also re-entrancy-guards itself as a backstop.
- `exporting: boolean` (Task 36) - whether an export pipeline is running (`Editor`'s lifted `exporting` state, the same one `TopBar` reads). Combined with `dur <= 0` into a local `locked` flag that disables Play, Trim In/Out/Reset, and the aspect chip - none of them should change what's rendering mid-export, and none are meaningful with no clip loaded yet.
- `trimmed: boolean` - whether the doc's trim range is currently narrower than the whole clip (`resolveTrim(doc.trim, dur)` != `[0, dur]`). Drives whether the Reset button renders at all.
- `onTrimIn: () => void` / `onTrimOut: () => void` - trim the clip's start/end straight to the current playhead (`useTrimActions` in `Editor.tsx`, which emits `set_trim` ops and guards each direction from collapsing the range - In ignores a click at/past the current out, Out ignores a click at/before the current in).
- `onResetTrim: () => void` - clears the trim back to the whole clip (`SetTrim { in_ms: 0, out_ms: 0 }`). The Reset button only renders when `trimmed` is true (nothing to reset otherwise) - dragging `TrimOverlay`'s edge handles on the timeline is still an alternate way to set the same trim range.
- `aspect: Aspect` / `onAspect: (aspect: Aspect) => void` - the doc's output aspect ratio and its setter (`SetAspect`). The chip cycles through all 5 `Aspect` values in `ASPECT_ORDER` on click, labelled via `ASPECT_LABEL`.
- `quality: number` / `onQuality: () => void` - the preview proxy height (480/720/1080) and its cycle handler (unrelated to `aspect`: this is the proxy-video transcode resolution, not the compositing canvas size).
- `muted: boolean` / `onMute: () => void` - audio mute toggle.
- `volume: number` / `onVolume: (v: number) => void` - the preview-audio volume as `0..100`, owned by `Editor`. The same value (divided by 100) is what `Stage` actually applies as the hidden `<audio>` element's gain via `useMediaPlayback` - it has to live above both components rather than as local state here.

### Behavior

**Locked state (Task 36).** `locked = exporting || dur <= 0`, computed once per render. Play, both trim buttons, the reset button, and the aspect chip all pass `disabled={locked}` (styled via `.e-play:disabled`/`.e-tbtn:disabled`/`.e-tg:disabled`/`.e-chip:disabled` - dimmed, `cursor: default`); their Motion `whileHover`/`whileTap` springs are also omitted while `locked` (spread conditionally) so a disabled button doesn't visually invite a press it will ignore. Skip-to-start/end, the quality chip, and mute are unaffected - they're safe (or still meaningful) during either state.

**Aspect chip.** `cycleAspect` finds `aspect`'s index in `ASPECT_ORDER` and calls `onAspect` with the next one (wrapping); the chip label comes from `ASPECT_LABEL[aspect]` (`"Source"`, `"16:9"`, `"9:16"`, `"1:1"`, `"4:3"`).

**Trim In / Out.** Two `.e-tbtn` buttons (`IconArrowBarToLeft`/`IconArrowBarToRight`, labelled "In"/"Out"), `disabled={locked}`, call `onTrimIn`/`onTrimOut` to cut the clip's start/end to wherever the playhead currently sits. A third button (`.e-tg on`, `IconX`, title "Reset the trim range"), also `disabled={locked}`, renders only when `trimmed` and calls `onResetTrim` - there's no separate disabled "pill" state for its own absence, the reset control simply isn't in the DOM until there's something to reset.

**Add zoom + AI Auto-director wand.** Both are `motion.button.e-tg` sharing the same tap/hover spring `.e-play` uses (`whileHover: scale 1.04`, `whileTap: scale 0.98`, `0.12s` tween, Task 36). Add-zoom (`IconZoomIn`, title "Add a zoom region here (Z)") calls `onAddZoom` unconditionally. The wand (`IconWand`, title "Run the AI director to auto-edit this clip") calls `onAutoedit`, `disabled={aiRunning}` - grayed out (`.e-tg:disabled`, `opacity: .4`) and inert (spring omitted too) for the whole director run, not just until the click registers. Carries `data-director-anchor="wand"` - `src/editor/director/targets.ts`'s `anchorPoint("wand")` uses this (alongside the AiPanel run button, which carries the same attribute) to find where the AI director's fake pointer should start a run from.

**Playback controls.** Skip-to-start (title "Jump to the start") calls `onSeek(0)`; Play/Pause (`disabled={locked}`, spring omitted while locked) calls `onPlay()` and shows `IconPlayerPause`/`IconPlayerPlay` by `playing` (both `fill="currentColor"` so they render as solid glyphs rather than outlines, and the play triangle carries a `marginLeft: 2` nudge to sit visually centered); skip-to-end (title "Jump to the end") calls `onSeek(dur)`.

**Mute button.** Shows `IconVolumeOff` when `muted` is true OR `volume === 0` (so a slider dragged all the way down reads as muted even before the mute button itself is clicked), otherwise `IconVolume`. The click handler is always `onMute` - a plain toggle that doesn't touch `volume`.

**Volume flyout.** Hovering `.e-volwrap` reveals (`AnimatePresence` fade+slide, no width animation) a `Slider` bound directly to the `volume`/`onVolume` props (`0..100`, displaying `0` while `muted` without altering the underlying `volume` value) - a real, `Editor`-owned control now rather than a local mock. Dragging it above `0` while `muted` also calls `onMute()`, so raising the slider from a muted state unmutes rather than silently changing a number nobody hears. **Hover bridge (Task 36):** `.e-volwrap` has no flex `gap` between the mute button and `.e-volflyout` - the same 6px of breathing room instead lives as `.e-volflyout`'s own `padding-left`, so the button and flyout sit flush with zero dead space between them and the wrap's hover state (which shows/hides the flyout) never has a gap to lose the pointer over while crossing from the button onto the slider.

### Notes

- Transport owns no edit-doc state directly - `trimmed`/`aspect`/`quality`/`muted`/`volume`/`exporting` are all derived/owned by `Editor.tsx` and passed down; only the volume flyout's open/closed flag (`showVolumeSlider`) is local. `volume` used to be a local mock but is real, lifted state as of this change.
