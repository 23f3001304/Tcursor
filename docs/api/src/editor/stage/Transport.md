# src/editor/stage/Transport.tsx

The transport bar rendered between the Stage and the Timeline. Left: a Trim pill (a status/reset control - the actual trim range is set by dragging `TrimOverlay`'s edge handles on the timeline) plus add-zoom / AI-autoedit / split-at-playhead tools. Center: skip-to-start / play-pause / skip-to-end and the time readout. Right: an aspect-ratio chip, a preview-quality chip, and mute + volume. Local state is limited to the volume flyout (`volume`, `showVolumeSlider`); every edit-affecting control calls back into `Editor`.

## Transport

```tsx
export function Transport({
  timeMs, dur, playing, onPlay, onSeek, onAddZoom, onAutoedit, onSplit,
  trimmed, onResetTrim, aspect, onAspect, quality, onQuality, muted, onMute,
}: {
  timeMs: number; dur: number; playing: boolean; onPlay: () => void; onSeek: (ms: number) => void;
  onAddZoom: () => void; onAutoedit: () => void; onSplit: () => void;
  trimmed: boolean; onResetTrim: () => void;
  aspect: Aspect; onAspect: (aspect: Aspect) => void;
  quality: number; onQuality: () => void;
  muted: boolean; onMute: () => void;
}): JSX.Element
```

Renders the transport bar.

### Props

- `timeMs: number` / `dur: number` - the playhead and total clip duration (ms); shown as `fmt(timeMs) / fmt(dur)`.
- `playing: boolean` / `onPlay: () => void` - play/pause state and toggle. *Behavior owned by the caller:* `Editor.tsx`'s `onPlay` snaps the playhead forward to the trim-in point when starting playback from outside the trim range (see `resolveTrim`).
- `onSeek: (ms: number) => void` - called with `0` (skip-to-start) or `dur` (skip-to-end).
- `onAddZoom: () => void` - adds a zoom at the playhead.
- `onAutoedit: () => void` - runs the AI auto-director.
- `onSplit: () => void` - adds a cut at the playhead.
- `trimmed: boolean` - whether the doc's trim range is currently narrower than the whole clip (`resolveTrim(doc.trim, dur)` != `[0, dur]`). Drives the Trim pill's highlighted "on" state and whether it's clickable.
- `onResetTrim: () => void` - clears the trim back to the whole clip (`SetTrim { in_ms: 0, out_ms: 0 }`). The pill is `disabled` when `!trimmed` (nothing to reset) - the actual trim is SET by dragging `TrimOverlay`'s edge handles on the timeline, not from this button.
- `aspect: Aspect` / `onAspect: (aspect: Aspect) => void` - the doc's output aspect ratio and its setter (`SetAspect`). The chip cycles through all 5 `Aspect` values in `ASPECT_ORDER` on click, labelled via `ASPECT_LABEL`.
- `quality: number` / `onQuality: () => void` - the preview proxy height (480/720/1080) and its cycle handler (unrelated to `aspect`: this is the proxy-video transcode resolution, not the compositing canvas size).
- `muted: boolean` / `onMute: () => void` - audio mute toggle.

### Behavior

**Aspect chip.** `cycleAspect` finds `aspect`'s index in `ASPECT_ORDER` and calls `onAspect` with the next one (wrapping); the chip label comes from `ASPECT_LABEL[aspect]` (`"Source"`, `"16:9"`, `"9:16"`, `"1:1"`, `"4:3"`).

**Trim pill.** Shows the `.on` class (a red-tinted highlight, see `editor.css`'s `.e-tbtn.on`) and "Reset trim range" as its title when `trimmed`; otherwise it's disabled with a hint to drag the timeline handles.

**Playback controls.** Skip-to-start calls `onSeek(0)`; Play/Pause calls `onPlay()` and shows `IconPlayerPause`/`IconPlayerPlay` by `playing`; skip-to-end calls `onSeek(dur)`.

**Volume flyout.** Hovering `.e-volwrap` reveals a `Slider` (Motion fade+slide, no width animation) driving local `volume` state; unmuting via the slider (`v > 0` while `muted`) calls `onMute()`.

### Notes

- Transport owns no edit-doc state directly - `trimmed`/`aspect`/`quality`/`muted` are all derived/owned by `Editor.tsx` and passed down; only the volume flyout's open/closed + slider value are local.
