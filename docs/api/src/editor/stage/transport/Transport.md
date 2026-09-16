# src/editor/stage/transport/Transport.tsx

The transport bar rendered between the Stage and the Timeline. Left: Trim In/Out buttons that cut the clip straight to the playhead, a Reset button that appears only once trimmed, then a divider plus add-zoom / AI-autoedit tools. Center: skip-to-start / play-pause / skip-to-end and the time readout. Right: an aspect-ratio chip, a preview-quality chip, the view-mode control (`ViewPicker`), and mute + a real volume flyout. This file is now the bar's contract and its assembly only: the three clusters it composes are [TransportTools](TransportTools.md) (left), [PlaybackGroup](PlaybackGroup.md) (centre) and [OutputGroup](OutputGroup.md) (right), each owning its own markup, its own local state and its own notes. The only thing computed here is `locked`, which all three read.

## Transport

**Add a text item (Batch 2c).** `onAddText: () => void`, threaded straight through to `TransportTools` exactly as `onAddZoom` is. It takes no kind here on purpose: `ClassicShell` closes over the default (`() => p.addText("title")`), so the transport stays a bar of one-click tools and the four-way choice lives where there is room for it, in the Effects panel's Add pills.

**Cut and Speed (T7).** Four more props, all threaded straight through to `TransportTools` and not otherwise read here: `clicks` (the recording's clicks on clip time, the range-less span's lookahead), `range`/`setRange` (the ruler's selection and the way to clear it) and `onApply` (the op sink). `timeMs` and `dur`, which the readout already took, are passed down as well - `TransportTools` needs both to compute the span.

**Time remap readout.** Three props: `outTimeMs` (output time, what the viewer will see: cuts skipped, speed applied), `outDur` (the exported length) and `plain` (no cuts and no speed spans). The big number is `outTimeMs` over `outDur`; clip time (`timeMs`, the raw recording's clock every lane still sits on) appears as `.e-time-clip` fine print only when `plain` is false. `dur` remains the seek range (jump to the end seeks to clip `dur`). The readout itself lives in `PlaybackGroup`; the props are declared here because this file is the bar's single contract with `Editor`.

```tsx
export const Transport: React.MemoExoticComponent<(props: {
  timeMs: number; dur: number; playing: boolean; onPlay: () => void; onSeek: (ms: number) => void;
  onAddZoom: () => void; onAddText: () => void; onAutoedit: () => void; aiRunning: boolean; exporting: boolean;
  trimmed: boolean; onTrimIn: () => void; onTrimOut: () => void; onResetTrim: () => void;
  aspect: Aspect; onAspect: (aspect: Aspect) => void;
  quality: number; onQuality: () => void;
  muted: boolean; onMute: () => void;
  volume: number; onVolume: (v: number) => void;
}) => JSX.Element>
```

Renders the transport bar. `React.memo`'d (render hygiene pass) - `timeMs` still ticks every frame during playback (the time readout genuinely needs it live), so this can't skip re-rendering entirely, but memo still avoids a re-render from unrelated `Editor` state as long as the caller passes stable callback props (`hooks/input/useEditorCallbacks.ts`).

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

**Locked state (Task 36).** `locked = exporting || dur <= 0`, computed once per render and passed to all three clusters. Play, both trim buttons, the reset button, and the aspect chip all pass `disabled={locked}` (styled via `.e-play:disabled`/`.e-tbtn:disabled`/`.e-tg:disabled`/`.e-chip:disabled` - dimmed, `cursor: default`). Play and the reset button are `motion.button`s whose `whileTap` spring is also omitted while `locked` (spread conditionally, `locked ? undefined : ...`) so a disabled button doesn't visually invite a press it will ignore; the trim buttons and the aspect chip have no Motion at all - their own hover-lift + `:active` press is plain CSS (design/premium-pass D6 leaves `.e-tbtn`/`.e-chip` alone, see `editor.css`). Skip-to-start/end, the quality chip, and mute are unaffected by locking - they're safe (or still meaningful) during either state.

**The bar wraps rather than overflowing (width audit, 2026-09-14).** Its three groups come to roughly 1000px of fixed-size controls and none of them can shrink - every button carries a pixel width and `.e-time` is `nowrap` - so on any window under about 1100px the old `height: 44px`, no-wrap bar ran straight past its own `max-width: calc(100% - 32px)` and the right group (view mode and the volume control) was cut off by `.editor`'s `overflow: hidden`. `.e-transport` is now `flex-wrap: wrap` with `min-height: 44px` instead of a fixed height, plus `justify-content`/`align-content: center` and a 5px row gap: a normal window still renders one row inside the same 44px box, and the 880px minimum window gets two centred rows instead of losing controls off the edge.

**Quieter secondary controls (look pass).** `.e-tbtn` and `.e-chip` lost their gradient fill and their 1px hover lift (`stage.css`). In a bar whose hero is Play, eight secondary controls should not each rise off the surface when the pointer crosses them; hover is now the same translucent `--e-divider` wash the neighbouring `.e-tg` buttons already used.

**Trim In / Out.** Two `.e-tbtn` buttons (`IconArrowBarToLeft`/`IconArrowBarToRight`, labelled "In"/"Out"), `disabled={locked}`, call `onTrimIn`/`onTrimOut` to cut the clip's start/end to wherever the playhead currently sits. A third button (`motion.button.e-tg on`, `IconX`, title "Reset the trim range"), also `disabled={locked}`, renders only when `trimmed` and calls `onResetTrim` - there's no separate disabled "pill" state for its own absence, the reset control simply isn't in the DOM until there's something to reset. Its press (`whileTap: { scale: 0.96 }` when not `locked`, `PLAY_SPRING`'s stiffness/damping) is the app-wide press spring (design/premium-pass D6) filling a gap the plain `.e-tbtn`/`.e-tg` buttons around it didn't have.

**Add zoom + AI Auto-director wand.** Both are `motion.button.e-tg` sharing a tap/hover spring (`TAP_SPRING`: `whileHover: scale 1.04`, `whileTap: scale 0.98`, `0.12s` tween, Task 36). Add-zoom (`IconZoomIn`, title "Add a zoom region here (Z)") calls `onAddZoom` unconditionally. The wand (`IconWand`, title "Run the AI director to auto-edit this clip") calls `onAutoedit`, `disabled={aiRunning || exporting}` (bug-sweep-2 Task 8, L3 added the `exporting` half - see `exporting`'s own note above) - grayed out (`.e-tg:disabled`, `opacity: .4`) and inert (spring omitted too) for the whole director run OR export, not just until the click registers. Carries `data-director-anchor="wand"` - `src/editor/director/targets.ts`'s `anchorPoint("wand")` uses this (alongside the AiPanel run button, which carries the same attribute) to find where the AI director's fake pointer should start a run from.

### Notes

- Transport owns no state at all - `trimmed`/`aspect`/`quality`/`muted`/`volume`/`exporting` are all derived/owned by `Editor.tsx` and passed down, and the one piece of local state on the bar (the volume flyout's open/closed flag) now lives inside `OutputGroup`.

`onDetectSilences` is threaded straight through to `TransportTools` like `clicks`, `range` and `onApply`.
