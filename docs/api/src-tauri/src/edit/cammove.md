# src-tauri/src/edit/cammove.rs

The camera-move keyframe (`CameraMove`) and its default roundness, moved out of `edit/model.rs` verbatim for headroom (2026-09-15). `edit::model` re-exports both, so every existing path still resolves; `EditDoc.camera_moves` is the list that carries them.

## CameraMove

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CameraMove {
    pub id: String, pub t_ms: u32, pub x: f32, pub y: f32, pub size: f32,
    #[serde(default = "default_cam_easing")] pub easing: String,
    #[serde(default = "default_cam_shape")] pub shape: String,
    #[serde(default = "default_cam_roundness")] pub roundness: f32,
}
```

One keyframe of the webcam PiP's position + size + shape track (`EditDoc.camera_moves`). An empty track is the default and is a no-op at render time - a doc with no `camera_moves` composites byte-identically to today.

- `id` - *stable string key (e.g. `"k0"`, `"k3"`) used to target a specific keyframe for update/removal without relying on list position.*
- `t_ms` - *the frame time this keyframe is pinned to.*
- `x` / `y` - *the PiP's center, as a fraction (`0.0`-`1.0`) of the output frame.*
- `size` - *the PiP's size, as a fraction of the output frame; the interpolator (`CameraMoveTrack`) derives the other dimension from the mode's aspect.*
- `easing` - *named easing curve (`"linear"`, `"smooth"`, `"spring"`) for the ramp into this keyframe; defaults to `"smooth"` when absent from JSON, matching `Zoom`/`LayoutSeg`'s back-compat pattern.*
- `shape` - *the keyframe's own webcam shape: `"layout"` (inherit the layout's - the serde default, so every keyframe written before shapes existed on 2026-09-14 reads unchanged), `"circle"`, `"rounded"` or `"rect"`. `CameraMoveTrack` folds it into the pose as a corner fraction and morphs it between keyframes like the rect.*
- `roundness` - *the `"rounded"` corner radius as a fraction of the panel's short side, `0.0`-`0.5`; ignored by the other shapes. Defaults to `DEFAULT_CAM_ROUNDNESS`.*

## DEFAULT_CAM_ROUNDNESS

```rust
pub const DEFAULT_CAM_ROUNDNESS: f32 = 0.12;
```

The corner radius a keyframe gets when switched to `"rounded"` before its slider is touched - visibly rounded at bubble sizes without reading as a pill. Also `roundness`'s serde default, and what `EditOp::AddCameraMove` fills in when the op carries none.

