# src/editor/hooks/stage/compositeLoopRefs.ts

The prop-type bundle for `useCompositeLoop`, moved out of that file when it hit its 200-line cap. A type only: no runtime code, no behaviour change, and the hook destructures this exact shape.

Context for the extraction (M5 T5): the plan's named extraction, ADDED-7, was to move the FX-overlay request block out of `useCompositeLoop.ts`. That had already shipped as `fxRequestTick.ts`, so the room for the caption draw came from the next-largest movable block instead - the ~30 lines of inline parameter types. Same purpose, different half of the file.

## CompositeLoopRefs

```ts
export interface CompositeLoopRefs {
  screenRef; webcamRef; audioRef; canvasRef;      // the hidden media and the canvas
  playRef; timeRef; onTimeRef;                     // the clock
  trackRef; layoutRef; layoutPresetsRef; layoutSegsRef; cameraMovesRef; zoomsRef; zoomSettingsRef;
  dragPoseRef; arrangingRef; clicksRef; effectsRef; clickfxRef; gradeRef;
  captionsRef; textsRef; capStyleRef; accentRef;   // the caption and text draws
  kindsRef; cursorRef; spritesRef; trailRef;
  dirtyRef; bgRef; spotSimRef; mapRef; exactRef; editGenRef;
}
```

Everything the rAF tick reads, as `RefObject`s so the loop mounts once (empty dependency array) and still sees live values. `useSyncRefs` produces most of them; `Stage.tsx` mirrors the rest directly with the same plain "useRef + assign each render" pattern.

### Notes

- `captionsRef` is `outDoc.captions` - the caption track on the OUTPUT clock, remapped by `remapDoc` exactly like zooms, layout segments and effects. The tick therefore reads it at `tOut`, not at clip time.
- `capStyleRef` is `doc.settings.captions` and `accentRef` is `doc.settings.ui.accent`: the same three inputs Rust `FrameRenderer::composite_at` hands `captiondraw::overlay`, so the preview and the export are fed identically.
- `textsRef` is `outDoc.texts`, on the OUTPUT clock like `captionsRef` and read at `tOut` for the same reason. With `accentRef` it is the whole of what `laidTexts` needs, which is exactly the pair `fx_pass` hands `textdraw::overlay`.
- `mapRef` is the clip-to-output clock map from `useTimeMap`; `spotSimRef` is owned by `Stage.tsx` because it is also reset on a paused effects-content edit, which the loop's own discontinuous-jump gate cannot see.
- `exactRef` / `editGenRef` carry the export's own frame for a resting playhead, keyed on the instant AND the edit generation.

### Used by

- `src/editor/hooks/stage/useCompositeLoop.ts` - the sole consumer; it is the hook's parameter type.
- `src/editor/stage/Stage.tsx` - builds the object structurally (it does not import the type).
