# src/hud/components/CamTile.tsx

The recording bar's live camera-preview tile. Split out of `Hud.tsx` (200-line cap) so gate-feedback item 4's saving-state handling (user-reported 2026-09-02: the live webcam preview kept showing, and the camera stayed live, in the saving bar) has a single, obvious home.

## CamTile

```ts
export function CamTile({ camRef, camOn, camLive, shape }: { camRef: RefCallback<HTMLVideoElement>; camOn: boolean; camLive: boolean; shape?: "wide" | "round" }): JSX.Element
```

### Props

- `camRef: RefCallback<HTMLVideoElement>` - `Hud`'s `cam.ref` (from `useWebcamPreview`), bound straight to the `<video>` element.
- `camOn: boolean` - `Hud`'s camera toggle state.
- `camLive: boolean` - `Hud`'s `cam.on` (from `useWebcamPreview`) - `true` only once a real stream is actually attached.
- `shape?: "wide" | "round"` - `"wide"` is the idle card's preview across its width (`.camtoggle.wide`, 160px tall), `"round"` the take pill's 44px circle (`.camtoggle.round`); unset is the original 52px square. Same `<video>`, same off-glyph, larger in the wide one.

### Behavior

Renders a `<video>` bound to `camRef`, plus a camera-off glyph overlay whenever `camOn && camLive` is false - covers "toggled off" AND "no camera / device lost / stream not yet acquired" with the same glyph. Owns no state - purely a rendering of the truth its props already carry.

### Notes

This component does NOT decide whether the camera stream itself is live - that is `useWebcamPreview`'s `enabled` argument, which `Hud.tsx` passes as `camOn && !saving` (item 4). Whether a tile renders at all is the caller's call: the idle card and the recording pill both show one, the saving pill shows none rather than a dead/off-state tile - the camera LED going off is a separate, already-covered effect of the stream actually being released. Swapping between the idle tile and the pill's round one remounts the `<video>`; `useWebcamPreview`'s callback ref re-attaches the stream to the new element, which is why it is a callback ref in the first place.

### Used by

- `src/hud/components/IdleCard.tsx` - `shape="wide"`, the preview across the card under its header.
- `src/hud/components/TakeBar.tsx` - `shape="round"`, at the recording pill's left end.
