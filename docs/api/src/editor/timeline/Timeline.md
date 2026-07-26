# src/editor/timeline/Timeline.tsx

Multi-track (Filmora-style) bottom timeline: an adaptive ruler, a filmstrip clip, and a scrolling stack of tracks - the zoom track, effect (spotlight) layer rows, and the system/mic audio waveforms - with a playhead spanning the clip. Zoom + effect pills drag/resize via `useRegionDrag`; selecting one opens the matching inspector.

## Timeline

```tsx
export function Timeline({ doc, timeMs, dur, onSeek, sel, onSel, onApply, thumbs, waves }: {
  doc: EditDoc; timeMs: number; dur: number; onSeek: (ms: number) => void;
  sel: string | null; onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  thumbs: string[]; waves: { system: string; mic: string };
}): JSX.Element
```

Renders the ruler, filmstrip, track stack, and spring-animated playhead.

### Props

- `doc: EditDoc` - the edit document; `.zooms` and `.effects` are rendered as draggable region pills.
- `timeMs: number` - playhead position (ms), owned by `Editor`.
- `dur: number` - clip duration (ms); all positions/widths are percentages of `dur`.
- `onSeek: (ms) => void` - called with the new time on click/drag of the track body.
- `sel: string | null` / `onSel` - selected region id (zoom or effect), lifted to `Editor`.
- `onApply: (op) => Promise<EditDoc | null>` - persists an edit op (a region drag commits one `update_zoom` / `update_effect`).
- `thumbs: string[]` - filmstrip thumbnail asset URLs (from `ensureThumbs`).
- `waves: { system, mic }` - waveform PNG asset URLs (from `ensureWaveform`); `""` hides that track.

### Behavior

**Seek.** The track body (`.e-tlbody`, which wraps the filmstrip + track stack) captures the pointer on `pointerdown` and `seekAt(clientX)` maps the x into `[0, dur]`. Region-pill pointerdowns `stopPropagation`, so they drag instead of seeking.

**Ruler.** `rulerTicks(dur)` (adaptive) renders `<span>`s in `.e-ruler` positioned at `(at / dur) * 100%`.

**Filmstrip.** `<Filmstrip thumbs={thumbs} />` fills the clip with the frame thumbnails.

**Zoom track.** `layoutRegions(doc.zooms)` assigns each zoom a `layer` (same greedy interval-partitioning as the effect layers below), and the timeline renders one `.e-zoomrow` per layer - so overlapping zooms stack on separate rows instead of colliding in the same lane. Each entry is a `motion.div.e-zblk`, positioned `left: (s/dur)*100%`, `width: max(2.5%, ((e-s)/dur)*100%)` where `s/e` come from the live `useRegionDrag` draft while dragging (layer-aware: `useRegionDrag` only clamps a drag against same-layer neighbours, so cross-layer overlap during a drag is allowed). Body = move; the two `.e-zh` handles = retime. Selected pills get `.sel`. Spring: `stiffness 480, damping 30`.

**Effect layers.** `layoutRegions(doc.effects)` assigns each effect a `layer`; the timeline renders one `.e-fxrow` per layer (so overlapping spotlights stack on separate rows), each pill an amber `.e-fxblk` driven by a second `useRegionDrag` (layer-aware clamp). Selecting opens the `EffectInspector`.

**Camera lane.** `<CameraLane>` renders `doc.camera_moves` as diamond keyframes in its own row, below the region tracks. The `onDrop` handler's `type === "cammove"` branch (dropped from the Camera Move pill) applies `add_camera_move` at the drop time with a centered default (`x:0.5, y:0.5, size:0.25`).

**Audio tracks.** `<AudioTrack>` for system + mic waveforms, below the editable tracks.

**Trim overlay.** `<TrimOverlay trim={doc.trim} dur={dur} trackRef={track} onApply={onApply} />` renders last (highest paint order) inside `.e-tlbody`: dims the head/tail outside `doc.trim`'s resolved range (`resolveTrim`) and provides two draggable edge handles that commit `SetTrim` on release. See `TrimOverlay.md`.

**Playhead.** A `motion.div.e-ph` at `left: pct%` inside `.e-tlbody` (so it shares the track body's width/origin and stays aligned with ticks + pills), animated with a fast spring (`stiffness 700, damping 42`), `initial={false}`.

### Notes

- `pct` is 0 when `dur` is 0 - no division-by-zero; the playhead sits at the left edge.
- The two `useRegionDrag` instances (zoom + effects) only attach window listeners while their own drag is active, so they never cross-talk.
