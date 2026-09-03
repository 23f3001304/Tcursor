# src/editor/stage/arrange/ArrangeOverlay.tsx

Arrange mode's interactive layer: the selected layout segment's two panels framed over the live composite, plus the snap guides and the camera-keyframe hint. Rendered by `Stage` only while `arrangeSeg` is non-null.

## ArrangeOverlay

```tsx
export function ArrangeOverlay({ seg, panels, camMoves, guideX, guideY, active, onPanelDown, onHideCam }: {
  seg: LayoutSeg; panels: Panels; camMoves: CameraMove[];
  guideX: number | null; guideY: number | null; active: PanelKind | null;
  onPanelDown: (e: React.PointerEvent, panel: PanelKind, corner: Corner | null) => void;
  onHideCam: () => void;
}): JSX.Element
```

### Geometry and hit-testing

Each frame is an absolutely-positioned div sized as a % of `.e-stage`, which is exactly the canvas' own displayed rect - the same trick `CamDragHandle` uses, so no letterbox math is needed here. That is also why hit-testing is the DOM's job rather than a pure function: the browser already knows which frame and which corner handle the pointer is over, and a JS hit-test would be a second copy of the panel geometry free to drift from the one being drawn. The cam frame renders after the screen frame, so **the cam always wins an overlap** - DOM order, not size. That is the right default (the cam is normally the small panel sitting on top of the screen), but it does mean a cam grown to cover the screen makes the screen frame unreachable on the stage; the way out is the inspector, or shrinking the cam back.

Body pointerdown = move; a corner handle's pointerdown = resize (the handle's own `onPanelDown` stops propagation, so the body never also sees it).

### Affordances

- **Frames** - 1.5px `--e-layout` outline (the Layout timeline lane's accent, matching how `.e-camdrag` borrows `--e-cam`), with `--e-line-strong` corner handles.
- **Cam x** - hides the webcam for this segment. Cam only: the screen can only be hidden from `LayoutInspector`, so the stage can never be blanked by one stray click. Rendered only while the SCREEN is visible (`canHideCam`) - otherwise the write is the "hide both panels" case Rust rejects, and because a rejected op still resolves, the button would look like it worked while leaving a phantom undo step and a pointless `rev` bump behind. Reachable today via the shipped Camera-only preset, whose screen panel is hidden.
- **Guides** - a 1px accent line on each snapped axis, faded in/out over 100ms by Motion (`AnimatePresence`; the timing constant is hoisted to module scope like `PRESS_SPRING` elsewhere). Only present while a drag is actually snapped - `useArrangeDrag` drops them on release.
- **Keyframe dots + hint** - any `camera_moves` keyframe inside `[seg.start_ms, seg.end_ms)` is drawn dimmed, with one `--e-dim` line: *"Keyframes refine the webcam inside this segment"*, pinned TOP-centre (the Toast pill and the director status pill both own bottom-centre over the stage, and this hint can be up for as long as the mode is). Informational only - T27's span semantics are unchanged by arranging, and this overlay changes nothing about them; it just stops the two systems reading as one.

Only panels above `VISIBLE_ALPHA` get a frame, so a hidden panel has nothing to grab (the inspector's switches are what bring it back).
