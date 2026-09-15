# src/editor/hooks/input/useMoveModeGuard.tsx

Owns the editor's "Move in preview" toggle and guards turning it OFF, since `camera_moves` keyframes override the static webcam controls (size/dock) within the span they own (Task 27) - so returning to static only fully works if the keyframes are cleared, otherwise the static sliders stay dead across that span while working everywhere else.

## useMoveModeGuard

```tsx
export function useMoveModeGuard(doc: EditDoc | null, applyOp: (op: EditOp) => Promise<EditDoc | null>,
  camDraftRef: RefObject<CamPose | null>): {
  moveMode: boolean;
  requestMoveMode: (next: boolean) => void;
  moveOffDialog: JSX.Element;
  moveOffOpen: boolean;
}
```

`requestMoveMode(next)` is the toggle handler wired to `CameraPanel`'s Switch: turning Move on sets the mode directly; turning it off *with* keyframes opens a `ConfirmDialog` ("your N keyframes will be removed") unless the user previously ticked "don't ask again" (persisted in `localStorage` under `tcursor_hide_move_off_warn`). Confirming clears every keyframe via one `remove_camera_move` per entry and turns the mode off, so the static size/dock sliders take effect again.

**Turning off also drops the unsaved drag** (`camDraftRef.current = null`, in `off`, on every path that ends the mode). *Why here:* the composite draws `camDraftRef` in preference to the keyframes, the span rule and the smart webcam action (`frameCamLayout`), and nothing else clears it when the mode ends - so a drag left behind kept the PiP on its dragged pose for the whole playback after Move mode was off, while the paused frame (the Rust one) looked right (bug sweep 2026-09-15). The other two clear triggers - the Add/Update keyframe button and a real playhead move - are unchanged. Render `moveOffDialog` somewhere in the editor tree (it is a fixed-position scrim). `moveOffOpen` (bug-sweep-2 Task 8, M4) is the same `pending` state `moveOffDialog`'s `open` prop already gets, re-exposed so `Editor` can fold this ConfirmDialog into its `modalOpen` check for `useEditorKeymap` - a keyboard shortcut must be just as inert behind THIS dialog as behind Export/Settings/Shortcuts.

**Render hygiene pass.** `clearKeyframes`/`requestMoveMode`/`confirm` are all `useCallback`'d now (deps bottom out in `doc`/`applyOp`, neither of which changes on a playhead tick) so `requestMoveMode`'s identity survives a tick - `Editor.tsx` builds its `onMoveMode` wrapper (passed to `EditorPanels`, `React.memo`'d) directly from it, so an unstable `requestMoveMode` here would have defeated that memo regardless of anything stabilized in `Editor.tsx` itself.
