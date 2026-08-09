# src/editor/hooks/useMoveModeGuard.tsx

Owns the editor's "Move in preview" toggle and guards turning it OFF, since `camera_moves` keyframes override the static webcam controls (size/dock) within the span they own (Task 27) - so returning to static only fully works if the keyframes are cleared, otherwise the static sliders stay dead across that span while working everywhere else.

## useMoveModeGuard

```tsx
export function useMoveModeGuard(doc: EditDoc | null, applyOp: (op: EditOp) => Promise<EditDoc | null>): {
  moveMode: boolean;
  requestMoveMode: (next: boolean) => void;
  moveOffDialog: JSX.Element;
}
```

`requestMoveMode(next)` is the toggle handler wired to `CameraPanel`'s Switch: turning Move on (or off with no keyframes) sets the mode directly; turning it off *with* keyframes opens a `ConfirmDialog` ("your N keyframes will be removed") unless the user previously ticked "don't ask again" (persisted in `localStorage` under `tcursor_hide_move_off_warn`). Confirming clears every keyframe via one `remove_camera_move` per entry and turns the mode off, so the static size/dock sliders take effect again. Render `moveOffDialog` somewhere in the editor tree (it is a fixed-position scrim).
