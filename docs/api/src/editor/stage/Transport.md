# src/editor/stage/Transport.tsx

The transport bar rendered between the Stage and the Timeline. Left: Trim In/Out buttons that cut the clip straight to the playhead, a Reset button that appears only once trimmed, then a divider plus add-zoom / AI-autoedit tools. Center: skip-to-start / play-pause / skip-to-end and the time readout. Right: an aspect-ratio chip, a preview-quality chip, and mute + a real volume flyout. Local state is limited to the volume flyout's open/closed flag (`showVolumeSlider`) - `volume` itself is now a prop owned by `Editor` (the same value `Stage` applies as real audio gain), not a local mock.

## Transport

```tsx
export function Transport({
  timeMs, dur, playing, onPlay, onSeek, onAddZoom, onAutoedit,
  trimmed, onTrimIn, onTrimOut, onResetTrim, aspect, onAspect, quality, onQuality, muted, onMute,
  volume, onVolume,
}: {
  timeMs: number; dur: number; playing: boolean; onPlay: () => void; onSeek: (ms: number) => void;
  onAddZoom: () => void; onAutoedit: () => void;
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
- `trimmed: boolean` - whether the doc's trim range is currently narrower than the whole clip (`resolveTrim(doc.trim, dur)` != `[0, dur]`). Drives whether the Reset button renders at all.
- `onTrimIn: () => void` / `onTrimOut: () => void` - trim the clip's start/end straight to the current playhead (`useTrimActions` in `Editor.tsx`, which emits `set_trim` ops and guards each direction from collapsing the range - In ignores a click at/past the current out, Out ignores a click at/before the current in).
- `onResetTrim: () => void` - clears the trim back to the whole clip (`SetTrim { in_ms: 0, out_ms: 0 }`). The Reset button only renders when `trimmed` is true (nothing to reset otherwise) - dragging `TrimOverlay`'s edge handles on the timeline is still an alternate way to set the same trim range.
- `aspect: Aspect` / `onAspect: (aspect: Aspect) => void` - the doc's output aspect ratio and its setter (`SetAspect`). The chip cycles through all 5 `Aspect` values in `ASPECT_ORDER` on click, labelled via `ASPECT_LABEL`.
- `quality: number` / `onQuality: () => void` - the preview proxy height (480/720/1080) and its cycle handler (unrelated to `aspect`: this is the proxy-video transcode resolution, not the compositing canvas size).
- `muted: boolean` / `onMute: () => void` - audio mute toggle.
- `volume: number` / `onVolume: (v: number) => void` - the preview-audio volume as `0..100`, owned by `Editor`. The same value (divided by 100) is what `Stage` actually applies as the hidden `<audio>` element's gain via `useMediaPlayback` - it has to live above both components rather than as local state here.

### Behavior

**Aspect chip.** `cycleAspect` finds `aspect`'s index in `ASPECT_ORDER` and calls `onAspect` with the next one (wrapping); the chip label comes from `ASPECT_LABEL[aspect]` (`"Source"`, `"16:9"`, `"9:16"`, `"1:1"`, `"4:3"`).

**Trim In / Out.** Two always-enabled `.e-tbtn` buttons (`IconArrowBarToLeft`/`IconArrowBarToRight`, labelled "In"/"Out") call `onTrimIn`/`onTrimOut` to cut the clip's start/end to wherever the playhead currently sits. A third button (`.e-tg on`, `IconX`, title "Reset trim range") renders only when `trimmed` and calls `onResetTrim` - there's no more disabled "pill" state, the reset control simply isn't in the DOM until there's something to reset.

**Playback controls.** Skip-to-start calls `onSeek(0)`; Play/Pause calls `onPlay()` and shows `IconPlayerPause`/`IconPlayerPlay` by `playing` (both `fill="currentColor"` so they render as solid glyphs rather than outlines, and the play triangle carries a `marginLeft: 2` nudge to sit visually centered); skip-to-end calls `onSeek(dur)`.

**Mute button.** Shows `IconVolumeOff` when `muted` is true OR `volume === 0` (so a slider dragged all the way down reads as muted even before the mute button itself is clicked), otherwise `IconVolume`. The click handler is always `onMute` - a plain toggle that doesn't touch `volume`.

**Volume flyout.** Hovering `.e-volwrap` reveals (`AnimatePresence` fade+slide, no width animation) a `Slider` bound directly to the `volume`/`onVolume` props (`0..100`, displaying `0` while `muted` without altering the underlying `volume` value) - a real, `Editor`-owned control now rather than a local mock. Dragging it above `0` while `muted` also calls `onMute()`, so raising the slider from a muted state unmutes rather than silently changing a number nobody hears.

### Notes

- Transport owns no edit-doc state directly - `trimmed`/`aspect`/`quality`/`muted`/`volume` are all derived/owned by `Editor.tsx` and passed down; only the volume flyout's open/closed flag (`showVolumeSlider`) is local. `volume` used to be a local mock but is real, lifted state as of this change.
