# src/editor/panels/CamShapeField.tsx

A camera-move keyframe's shape control, shared by the two places a keyframe is edited - the Move-mode field (`CameraMoveField.md`, writing the keyframe at the playhead) and the keyframe inspector (`CameraMoveInspector.md`, writing the selected keyframe) - so the two cannot offer different shapes.

## CAM_MOVE_SHAPES

```ts
export const CAM_MOVE_SHAPES: [CamMoveShape, string][] = [["layout", "Layout"], ["circle", "Circle"], ["rounded", "Rounded"], ["rect", "Rect"]];
```

The four shapes a keyframe can carry, in picker order with their labels. "Layout" is first and the default: it means "whatever the layout's webcam shape is", which is what every keyframe made before shapes existed reads as (Rust `CameraMove::shape`'s serde default).

## CamShapeField

```tsx
export function CamShapeField({ shape, roundness, onShape, onRoundness }: {
  shape: CamMoveShape; roundness: number;
  onShape: (v: CamMoveShape) => void; onRoundness: (v: number) => void;
}): JSX.Element
```

### Props

- `shape` / `roundness` - the keyframe's current values (`CameraMove.shape`, `.roundness`).
- `onShape` / `onRoundness` - one callback per field; each caller turns it into its own `update_camera_move`/`commitCamKeyframe` patch.

### Behavior

A four-way `Segmented` (aria "Keyframe Shape") and, for Rounded only, a "Corner Roundness" `Slider` on `SLIDERS.cam_radius` (0-0.5, a fraction of the panel's short side - the same slider the Layouts panel uses for the static webcam's `cam_radius`, so the two roundness controls mean the same number). Circle and Rect have nothing to tune, so the slider is not shown for them rather than shown disabled.
