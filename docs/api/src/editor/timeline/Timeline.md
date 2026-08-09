# src/editor/timeline/Timeline.tsx

Multi-track (Filmora-style) bottom timeline: an adaptive ruler, a filmstrip clip, and a scrolling stack of LABELED lanes (Task 36, restructured in the T36 fix-up below) - the zoom track, effect (spotlight) layer rows, layout-segment rows, the camera-move lane, and the system/mic audio waveforms - with a playhead spanning the clip. Zoom/effect/layout pills drag/resize via `useRegionDrag` and share one row-rendering component, `RegionRows.tsx`; selecting one opens the matching inspector.

## Timeline

**Layout-pill fade ramps.** Each layout pill sets two CSS custom properties inline, `--fin` and `--fout`, from `transitionRampPct` of its own `transition_ms`/`transition_out_ms` against its span (passed to `RegionRows` as its `extraStyle` callback). `.e-layblk`'s `::before`/`::after` render those as static gradients at the pill's two ends, so the pill reads as long as its fades actually are. Static CSS, no Motion - nothing here animates. A hard cut is `0%` wide and therefore invisible, so a pill with no transitions looks exactly as it always has.

```tsx
export function Timeline({ doc, timeMs, dur, playing, onSeek, sel, onSel, onApply, thumbs, waves, wavesReady }: {
  doc: EditDoc; timeMs: number; dur: number; playing: boolean; onSeek: (ms: number) => void;
  sel: string | null; onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  thumbs: string[]; waves: { system: string; mic: string }; wavesReady: boolean;
}): JSX.Element
```

Renders the ruler, filmstrip, labeled track stack, and spring-animated playhead.

### Props

- `doc: EditDoc` - the edit document; `.zooms`/`.effects`/non-`"screen"` `.layout` segments are rendered as draggable region pills, `.camera_moves` as keyframes.
- `timeMs: number` - playhead position (ms), owned by `Editor`.
- `dur: number` - clip duration (ms); all positions/widths are percentages of `dur`.
- `playing: boolean` - suppresses the playhead's spring transition (`duration: 0`) while actually playing, so it tracks the video frame-exact instead of trailing behind a spring during continuous playback; a scrub (not playing) still eases.
- `onSeek: (ms) => void` - called with the new time on click/drag of the track body.
- `sel: string | null` / `onSel` - selected region id (zoom, effect, layout segment, or camera keyframe), lifted to `Editor`.
- `onApply: (op) => Promise<EditDoc | null>` - persists an edit op (a region drag commits one `update_zoom` / `update_effect` / `update_layout_seg`).
- `thumbs: string[]` - filmstrip thumbnail asset URLs (from `ensureThumbs`).
- `waves: { system, mic }` - waveform PNG asset URLs (from `ensureWaveform`); `""` hides that track once resolved.
- `wavesReady: boolean` - whether the `waves` fetch has resolved (`useEditorData`). Passed to each `AudioTrack` as `loading={!wavesReady}` so a still-fetching waveform shows a shimmer skeleton instead of silently hiding the same as a genuinely-absent one.

### Behavior

**Seek.** The track body (`.e-tlbody`, which wraps the filmstrip + track stack) captures the pointer on `pointerdown`, sets a `scrubbing` ref true, and `seekAt(clientX)` maps the x into `[0, dur]`. `onPointerMove` only calls `seekAt` while `scrubbing.current` is true (and the primary button is held) - `onPointerUp`/`onLostPointerCapture` clear it. Region pills (`useRegionDrag.beginDrag`), trim handles, and camera keyframes all `stopPropagation` on their own `pointerdown`, so the body's handler never runs for them and `scrubbing` never flips true - a drag anywhere on those never chases the playhead, only a pointerdown that actually started on the bare body does. The playhead's own top handle (see below) is the one exception: it deliberately does NOT stop propagation, so grabbing it bubbles into this same handler for free.

**Ruler.** `rulerTicks(dur)` (adaptive) renders `<span>`s in `.e-ruler` positioned at `(at / dur) * 100%`.

**Filmstrip.** `<Filmstrip thumbs={thumbs} />` fills the clip with the frame thumbnails.

**Lane label gutter (Task 36, restructured in a follow-up fix - see "Why a sibling gutter, not a nested label" below).** A `lanes` array (`{key, label, heightPx, body}`) is built ONCE per render - one entry per visible lane (zoom/FX/layout only when they have rows; Camera always; Audio when loading or at least one source exists) - and mapped TWICE: once into `.e-lanegutter`'s label rows, once into `.e-tracks`' row bodies. Building it once and mapping it twice (rather than two separately-conditioned blocks) means the two columns can never drift out of sync with each other - the same array index always means the same lane in both places. Each gutter slot's `heightPx` is computed via the shared `laneHeight(rows, rowH)` helper (`ROW_H=32`/`AUDIO_ROW_H=22`/`GAP=6`, the exact numbers `editor.css` uses for `.e-zoomrow`/`.e-audiorow` etc.), so a lane's label slot is always exactly as tall as that lane's real rendered content, computed from the same row-count variables rather than duplicated as a second source of truth.

**Why a sibling gutter, not a nested label (fix-up).** The first cut of this feature positioned each lane's label `position: absolute; right: 100%` relative to the LANE itself, nested inside `.e-tracks`. That rendered nothing: `.e-tracks` has `overflow-y: auto` (for its vertical scroll when many lanes stack), which per the CSS spec forces its `overflow-x` to also compute as non-`visible` - and overflow clipping is DOM-containment-based, not geometry-based, so an absolutely-positioned descendant escaping its own parent's box via `right: 100%` is still clipped by any ANCESTOR's overflow, including `.e-tracks` two levels up. The fix moves the gutter (`.e-lanegutter`) to be a true SIBLING of `.e-tracks` (both children of `.e-trackswrap`, which is `position: relative`) - as a sibling it is not a descendant of `.e-tracks` at all, so its overflow never applies to it. `.e-lanegutter` is itself `position: absolute` within `.e-trackswrap` (`right: 100%` off `.e-trackswrap`'s own left edge, into `.e-timeline`'s widened left padding), so it takes NO width away from `.e-trackswrap`/`.e-tracks` - `.e-tracks` stays exactly 100% of `.e-tlbody`'s width, which is the SAME reference `seekAt` and `useRegionDrag`'s pointer-delta-to-ms conversion both use (`track.current.getBoundingClientRect()`) - completely unaffected by any of this. The one thing a sibling doesn't get for free is scroll: `.e-tracks`' `onScroll` (`onTracksScroll`) mirrors its `scrollTop` onto `gutterRef.current.scrollTop` on every scroll event, keeping the two columns visually locked together even though they're separate scroll contexts (`.e-lanegutter` has its own `overflow-y: hidden` - clippable but not user-scrollable, exactly what a programmatically-driven mirror needs).

**Zoom / FX / Layout lanes.** `layoutRegions(doc.zooms | doc.effects | doc.layout.filter(non-screen))` assigns each region a `layer` (greedy interval-partitioning), and each lane renders one row per layer via the shared `<RegionRows>` (`./RegionRows.tsx`, Task 36 - extracted so the three near-identical ~25-line pill blocks aren't triplicated) so overlapping regions stack on separate rows instead of colliding in the same lane. Each pill is a `motion.div` carrying `data-region-id` (zoom/FX only actually get looked up - `src/editor/director/targets.ts`'s `pillPoint` finds a just-applied zoom/effect's real DOM position for the AI director's fake pointer; layout pills carry the attribute too but nothing queries it), positioned `left: (s/dur)*100%`, `width: max(2.5%, ((e-s)/dur)*100%)` where `s/e` come from the live `useRegionDrag` draft while dragging (layer-aware: `useRegionDrag` only clamps a drag against same-layer neighbours, so cross-layer overlap during a drag is allowed). Body = move; the two `.e-zh` handles = retime. Selected pills get `.sel`. `RegionRows` takes each lane's icon+label as a `renderLabel` render-prop (zoom: `IconZoomIn` + `{scale}x`; FX: `IconBulb` + static "Spotlight"; layout: `IconAspectRatio` + `prettyLayout(layout)`) and, for layout only, an `extraStyle` callback for the `--fin`/`--fout` fade-ramp vars above. Selecting a layout pill opens `LayoutInspector`; FX opens `EffectInspector`.

**Tiny pills (Task 36).** `.e-zblk`/`.e-fxblk`/`.e-layblk` set `container-type: inline-size` and `min-width: 18px` (editor.css) - below a `34px` resolved width, a `@container` query hides `.e-zlabel` entirely (rather than letting it clip mid-glyph) while the pill itself stays at its `18px` floor, so a very short region is still visible and its full hit target (body drag + both edge handles) stays usable.

**Camera lane.** `<CameraLane>` renders `doc.camera_moves` as diamond keyframes in its own row, below the region lanes. The `onDrop` handler's `type === "cammove"` branch (dropped from the Camera Move pill) applies `add_camera_move` at the drop time with a centered default (`x:0.5, y:0.5, size:0.25`).

**Audio tracks.** `<AudioTrack src={waves.system} kind="system" loading={!wavesReady} />` and the `mic` counterpart, below the editable lanes, inside the shared "Audio" lane entry. `audioRows` (used for that lane's gutter height) mirrors `AudioTrack`'s own render-or-null rule: `!wavesReady` means both sources render as shimmers (2 rows), otherwise one row per source that actually has a `src`.

**Trim overlay.** `<TrimOverlay trim={doc.trim} dur={dur} trackRef={track} onApply={onApply} />` renders last (highest paint order) inside `.e-tlbody`: dims the head/tail outside `doc.trim`'s resolved range (`resolveTrim`) and provides two draggable edge handles that commit `SetTrim` on release. See `TrimOverlay.md`.

**Playhead.** A `motion.div.e-ph` at `left: pct%` inside `.e-tlbody` (so it shares the track body's width/origin and stays aligned with ticks + pills), transitioning instantly while `playing` else a `0.12s` tween, `initial={false}`. `.e-ph` itself stays `pointer-events: none` (body scrubbing must pass through it untouched), but its `<i>` flag marker sets `pointer-events: auto` and `cursor: grab` (Task 36) - the ONE interactive part of the playhead. Grabbing it fires a native `pointerdown` that bubbles straight up through the `pointer-events: none` ancestor to `.e-tlbody`'s own scrub handler (bubbling isn't blocked by an ancestor's `pointer-events: none`, only its own hit-testing is), so dragging the flag scrubs exactly like dragging the body - no extra event wiring needed. It grows slightly on hover (`transform: scale(1.3)`) to invite the grab.

### Notes

- `pct` is 0 when `dur` is 0 - no division-by-zero; the playhead sits at the left edge.
- The three `useRegionDrag` instances (zoom, effects, layout) only attach window listeners while their own drag is active, so they never cross-talk.
- See `RegionRows.md` for the shared pill-row renderer this file delegates to. There is no longer a separate `Lane` component - the gutter-vs-clipping fix (above) made the label and the row body two genuinely different DOM subtrees (sibling columns, not a wrap-both-in-one-box abstraction), so the split lives directly in this file's `lanes` array instead.
