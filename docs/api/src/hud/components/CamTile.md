# src/hud/components/CamTile.tsx

The recording bar's live camera-preview tile. Split out of `Hud.tsx` (200-line cap) so gate-feedback item 4's saving-state handling (user-reported 2026-09-02: the live webcam preview kept showing, and the camera stayed live, in the saving bar) has a single, obvious home.

## CamTile

```ts
export function CamTile({ camRef, camOn, camLive }: { camRef: RefCallback<HTMLVideoElement>; camOn: boolean; camLive: boolean }): JSX.Element
```

### Props

- `camRef: RefCallback<HTMLVideoElement>` - `Hud`'s `cam.ref` (from `useWebcamPreview`), bound straight to the `<video>` element.
- `camOn: boolean` - `Hud`'s camera toggle state.
- `camLive: boolean` - `Hud`'s `cam.on` (from `useWebcamPreview`) - `true` only once a real stream is actually attached.

### Behavior

Renders a `<video>` bound to `camRef`, plus a camera-off glyph overlay whenever `camOn && camLive` is false - covers "toggled off" AND "no camera / device lost / stream not yet acquired" with the same glyph. Owns no state - purely a rendering of the truth its props already carry.

### Notes

This component does NOT decide whether the camera stream itself is live - that is `useWebcamPreview`'s `enabled` argument, which `Hud.tsx` passes as `camOn && !saving` (item 4). This component only decides whether to render a TILE at all: `Hud.tsx` renders `<CamTile>` itself inside `{!saving && ...}`, hiding it completely while saving rather than showing a dead/off-state tile - the camera LED going off is a separate, already-covered effect of the stream actually being released.

### Used by

- `src/hud/Hud.tsx` - `{!saving && <CamTile camRef={cam.ref} camOn={camOn} camLive={cam.on} />}` in the recording row, unchanged across the idle and recording states.
