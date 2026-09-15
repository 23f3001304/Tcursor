# src/editor/timeline/Timeline.tsx

Multi-track (Filmora-style) bottom timeline: an adaptive ruler (`TimelineRuler.tsx`), a filmstrip clip, and a scrolling stack of LABELED lanes (Task 36, restructured in the T36 fix-up below) - the Time lane, the zoom track, effect (spotlight) layer rows, the captions lane (M5), layout-segment rows, the camera-move lane, and the system/mic audio waveforms - with a playhead spanning the clip. Zoom/effect/caption/layout pills drag/resize via `useLaneDrag` (`useLaneDrag.ts`, which wraps `useRegionDrag` and folds in the row-count arithmetic) and share one row-rendering component, `RegionRows.tsx`; selecting one opens the matching inspector.

## Timeline

```tsx
export const Timeline: React.MemoExoticComponent<(props: {
  doc: EditDoc; timeMs: number; dur: number; playing: boolean; onSeek: (ms: number) => void;
  sel: string | null; onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  thumbs: string[]; waves: { system: string; mic: string }; wavesReady: boolean;
  hasWebcam: boolean;
  layoutPresets: LayoutPresets | null;
  range: Range | null; setRange: (r: Range | null) => void;
}) => JSX.Element>
```

Renders the ruler, filmstrip, labeled track stack, and spring-animated playhead. Which lanes that stack holds, and what each one renders, comes from `useTimelineLanes(...)` (`useTimelineLanes.md`) - this file maps the array it returns twice, once into the gutter's labels and once into the rows. `React.memo`'d - see "Render hygiene" below.

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
- `hasWebcam: boolean` - `hasWebcamSignal(layout)` (`../model/editorData.ts`), passed straight through to `CameraLane` (see its own doc) to gate the empty-lane hint off for a webcam-less recording.
- `range: [number, number] | null` / `setRange` (time remap, T7) - the ruler's Shift+drag selection in clip ms, owned by `Editor.tsx` and threaded through `SlotProps`. The timeline only DRAWS it (`RangeOverlay`) and lets the ruler edit it; the transport's Cut and Speed are what act on it. See `useRangeSelect.md`.
- `layoutPresets: LayoutPresets | null` (T34 L4) - the Layout lane's thumbnail source: `useLayoutLaneRegions` (`LayoutLane.tsx`) resolves each layout pill's own panels through it (`resolvedPanelsFor`, T34 L2) so its pill can draw a small schematic of what it actually looks like, not just its preset's name. `null` until `useEditorData`'s first `previewLayouts` fetch lands - every pill falls back to a plain aspect-ratio icon until then (`LayoutLane.tsx`).

### Behavior

**Seek.** The track body (`.e-tlbody`, which wraps the filmstrip + track stack) captures the pointer on `pointerdown`, sets a `scrubbing` ref true, and `seekAt(clientX)` maps the x into `[0, dur]` and calls `onSeek` directly (immediate - no coalescing on the initial jump-to-click). `onPointerMove` instead calls `scheduleSeek` (from `useRafCoalesced`, `../hooks/stage/useRafCoalesced.ts`) while `scrubbing.current` is true (and the primary button is held) - this coalesces a pointermove burst (which can outpace the display refresh rate on a high-poll-rate mouse/trackpad) down to at most one `seekAt` per animation frame, always with the latest x. `onPointerUp`/`onLostPointerCapture` clear `scrubbing` AND call `flushSeek()`, so the exact release position lands immediately instead of waiting out the next frame. Region pills (`useRegionDrag.beginDrag`), trim handles, and camera keyframes all `stopPropagation` on their own `pointerdown`, so the body's handler never runs for them and `scrubbing` never flips true - a drag anywhere on those never chases the playhead, only a pointerdown that actually started on the bare body does. The playhead's own top handle (see below) is the one exception: it deliberately does NOT stop propagation, so grabbing it bubbles into this same handler for free.

**Drop (insert from the palette).** `.e-tlbody` also carries `onDragOver` (`preventDefault` + `dropEffect = "copy"` - without the `preventDefault` the browser refuses the drop) and `onDrop`, which reads the `text/plain` payload the `EffectPills.tsx` cards set on `dragstart` and switches on it: `"zoom"` -> `add_zoom`, `"spotlight"` -> `add_effect`, `"layout"` -> `add_layout_seg`, `"cammove"` -> `add_camera_move`, each at the drop x mapped into `[0, dur]` by the same rect math `seekAt` uses.

*This only works because `src-tauri/tauri.conf.json`'s main window sets `"dragDropEnabled": false`.* That key defaults to **true**, which puts Tauri's own native OS-level drag-drop handler in front of the webview: `dragstart` still fires on the palette card (the drag is real, and the OS reports one item), but the webview page never receives `dragover`/`drop`, so this handler simply never ran and pills could not be dragged onto the timeline at all. Nothing in the app listens for Tauri's native file-drop events (`onDragDropEvent`/`tauri://drag-*`), so turning the native path off costs nothing. It is a build-time config value - changing it needs a rebuild, not a page reload.

**Ruler (moved out, time remap T7).** `<Ruler dur trackRef onSeek range setRange />` (`./lanes/TimelineRuler.tsx`) renders `rulerTicks(dur)` exactly as this file used to, and adds two gestures of its own: a plain drag scrubs (the same `seekAt`/`scheduleSeek`/`flushSeek` contract as the body), a Shift+drag selects a range. `seekAt` and its rAF coalescing moved to that file as `useSeek` and are now called from BOTH surfaces, so the ruler and the track body cannot drift into two different x-to-ms mappings - both measure `trackRef` (`.e-tlbody`), which is the same content width as `.e-ruler`. The extraction was what made room for the Time lane and the cut overlay inside this file's 200-line cap.

**Cut overlay (time remap T7).** `<CutOverlay cuts dur sel onSel />` renders inside `.e-trackswrap` as a SIBLING of `.e-tracks`, not inside the Time lane's row and not inside the scroll container: a cut removes time from every lane at once, so it is drawn across the whole stack (the trim overlay's idiom, one plane down), and a sibling keeps it aligned with the lane viewport once that viewport scrolls. See `CutOverlay.md`.

**Range overlay (time remap T7).** `<RangeOverlay range dur />` renders inside `.e-tlbody` alongside `TrimOverlay`, so the selection reads against the lanes it will act on rather than only against the 18px ruler the gesture started on.

**Filmstrip.** `<Filmstrip thumbs={thumbs} />` fills the clip with the frame thumbnails. The lane is 80px tall (1.5x its old 54px) and holds NINE tiles instead of sixteen - `filmstripPlan.ts` derives both numbers and `useEditorData` requests exactly them, so a tile is a real 16:9-ish frame generated at the height it is drawn at rather than an upscaled 64px sliver. See `filmstripPlan.md` for the arithmetic and for the Rust constants it has to stay in step with.

**Stylesheet.** `Timeline.tsx` imports `./timeline.css`, which now holds every timeline rule this folder's components use (they moved out of `editor.css` wholesale in the timeline-readability pass, the way `panels.css` rides along with `PanelHeader.tsx`). `editor.css` keeps only the `.editor` token block the sheet draws from.

**Lane identity (timeline-readability pass, replaces Task D2's band parity).** BOTH mappings put the same `` `e-lane-${l.key}` `` class on their wrapper (`time`/`zoom`/`fx`/`captions`/`layout`/`camera`/`audio`) - a primitive string computed on a wrapper `Timeline` itself owns, never passed into a memoized child as a prop. `timeline.css` hangs one custom property off that class, `--lane-accent` (`--e-speed`/`--e-zoom`/`--e-fx`/`--e-layout`/`--e-cam`, and `--e-mut` for Audio, which has no accent of its own and no selectable content either), and both columns read it: the gutter as the label's 6px dot, the track body as a 5% tint of the whole `.e-lanerows` band plus a `--e-divider` hairline drawn inside the 6px gap by a pseudo-element (zero layout cost, so the two columns keep the identical heights `laneHeight` computes). This replaces the old index-parity `.e-band-a`/`.e-band-b` shade, which told you a lane was *not* its neighbour but never which lane it was.

**Why a sibling gutter, not a nested label (fix-up).** The first cut of this feature positioned each lane's label `position: absolute; right: 100%` relative to the LANE itself, nested inside `.e-tracks`. That rendered nothing: `.e-tracks` has `overflow-y: auto` (for its vertical scroll when many lanes stack), which per the CSS spec forces its `overflow-x` to also compute as non-`visible` - and overflow clipping is DOM-containment-based, not geometry-based, so an absolutely-positioned descendant escaping its own parent's box via `right: 100%` is still clipped by any ANCESTOR's overflow, including `.e-tracks` two levels up. The fix moves the gutter (`.e-lanegutter`) to be a true SIBLING of `.e-tracks` (both children of `.e-trackswrap`, which is `position: relative`) - as a sibling it is not a descendant of `.e-tracks` at all, so its overflow never applies to it. `.e-lanegutter` is itself `position: absolute` within `.e-trackswrap` (`right: 100%` off `.e-trackswrap`'s own left edge, into `.e-timeline`'s widened left padding), so it takes NO width away from `.e-trackswrap`/`.e-tracks` - `.e-tracks` stays exactly 100% of `.e-tlbody`'s width, which is the SAME reference `seekAt` and `useRegionDrag`'s pointer-delta-to-ms conversion both use (`track.current.getBoundingClientRect()`) - completely unaffected by any of this. The one thing a sibling doesn't get for free is scroll: `.e-tracks`' `onScroll` (`onTracksScroll`) mirrors its `scrollTop` onto `gutterRef.current.scrollTop` on every scroll event, keeping the two columns visually locked together even though they're separate scroll contexts (`.e-lanegutter` has its own `overflow-y: hidden` - clippable but not user-scrollable, exactly what a programmatically-driven mirror needs).

**Trim overlay.** `<TrimOverlay trim={doc.trim} dur={dur} trackRef={track} onApply={onApply} />` renders last (highest paint order) inside `.e-tlbody`: dims the head/tail outside `doc.trim`'s resolved range (`resolveTrim`) and provides two draggable edge handles that commit `SetTrim` on release. See `TrimOverlay.md`.

**Playhead.** `<Playhead pct playing dragging />` (`Playhead.tsx`), rendered inside `.e-tlbody` so it shares the track body's width and origin and stays aligned with ticks and pills. Timeline hands it three values and owns none of its rendering: the line, the glow and the drag ripple all live there, and so do the wave-motif restyle (the triangular grab head is now the brand's dot - the mark rotated 90 degrees) and the reduced-motion rule (the ripple alone is dropped). It was extracted when the motif added the ripple, because this file sits at its line cap.

`dragging` is `phDragging` - a small piece of REAL state (`useState`, not the `scrubbing` ref), set `true`/`false` alongside `scrubbing.current` in the track body's own pointerdown/up/lost-capture handlers - so the glow lights only while a scrub (of any kind, including a body click-drag, not just a grab of the head itself) is actually in progress. `scrubbing` itself stays a ref, so a pointermove burst still never triggers a Timeline re-render. The head remains the ONE interactive part of an otherwise `pointer-events: none` playhead, and still does NOT call `stopPropagation`, so its `pointerdown` bubbles straight up through the `pointer-events: none` ancestor to `.e-tlbody`'s scrub handler (bubbling is not blocked by an ancestor's `pointer-events: none`, only its own hit-testing is), scrubbing exactly like dragging the body - no extra event wiring needed.

### Notes

- `pct` is 0 when `dur` is 0 - no division-by-zero; the playhead sits at the left edge.
- The four `useLaneDrag` instances (zoom, effects, captions, layout) only attach window listeners once per drag (not once per pointermove - see `useRegionDrag.md`), so they never cross-talk and never churn listeners while dragging.
- See `RegionRows.md` for the shared pill-row renderer this file delegates to. There is no longer a separate `Lane` component - the gutter-vs-clipping fix (above) made the label and the row body two genuinely different DOM subtrees (sibling columns, not a wrap-both-in-one-box abstraction), so the split lives directly in this file's `lanes` array instead.
- See `useLaneDrag.md` for the per-lane drag + row-count call each pill lane makes, and `lanes/CaptionLane.md` for the Captions lane's own region hook and label (M5 T4) - both are extractions out of this file, made so a fourth lane fit under its line cap.
- See `lanes/LayoutLane.md` for the Layout lane's own region-resolving hook, `renderLabel`/`extraStyle` and the `LayoutThumb` they draw, and `model/arrThumb.md` for that thumbnail's geometry (T34 L4) - all split out of this file so it didn't have to grow past its own line cap to add the feature.

### Render hygiene

`Timeline` itself is `React.memo`'d, and so are its heaviest children (`Filmstrip`, `AudioTrack`, `RegionRows`, `CameraLane`) - so a re-render of `Timeline` (a real prop change, e.g. a playhead tick from `timeMs`) does not automatically cascade into re-rendering lanes that have nothing to do with the change. The derived values that make that hold are built in `useTimelineLanes.tsx` and listed under its own "Render hygiene" - see `useTimelineLanes.md`.

### `data-ui-fx="off"` on `.e-tracks` (micro-interaction pass, 2026-09-14)

The track stack carries `data-ui-fx="off"`, opting every lane, pill, handle and keyframe inside it out of the editor's click ripple ([ripples](../effects/ripples.md), [InterfaceEffects](../effects/InterfaceEffects.md)).

`.e-tracks` is a drag surface, not a control surface: a pointerdown on it is the start of a scrub, a pill drag, or a handle drag. A ripple would fire on the first frame of every gesture, and a drag that travels 300px would leave its bloom at the point it began.

It is `.e-tracks` specifically and not `.e-tlbody`, so the ruler above it and the transport around it keep their ripples - those really are clicks. Note that the overlay listens in the **capture** phase precisely because every draggable child here stops its own `pointerdown`; the opt-out attribute, not propagation, is what silences this stack.
