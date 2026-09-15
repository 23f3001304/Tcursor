# src/editor/inspectors/CaptionInspector.tsx

The selected caption's inspector: what it says, when it says it, and the three structural edits that only make sense on a caption. Routed by `shell/PropertiesSlot.tsx` like every other inspector, so selecting a caption pill (or a transcript row) opens it in the same sidebar a zoom or a cut opens in.

## CaptionInspector

```tsx
export function CaptionInspector({ caption, captions, dur, timeMs, onApply, onClose }: {
  caption: Caption; captions: Caption[]; dur: number; timeMs: number;
  onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}): JSX.Element
```

Four sections, each applying exactly one `EditOp`, so every edit is one undo step and the preview re-draws from the doc - the same live-edit contract `ZoomInspector` has.

| section | op |
| --- | --- |
| Text | `update_caption { text }`, on commit only (see `CaptionTextField.md`) |
| Timing | `update_caption { start_ms }` / `{ end_ms }`, through the shared `TimingRow` |
| Split | `split_caption { at_ms }` |
| Merge | `merge_captions { id }` |
| header trash | `remove_caption { id }`, then deselect |

Times are CLIP ms, the clock every pill on the timeline sits on (`CutInspector` says the same); the stage evaluates the remapped copy, so a caption stays under the words it belongs to even after a cut lands earlier in the recording.

### Split, at a word

While the caption still has its ASR word timings, Split is a row of word chips - one per boundary `captionEdit::splitPoints` allows - and pressing one cuts there. The chip's label is the word the SECOND caption would start with, which is the question the user is actually answering.

A caption typed by hand, or one whose timings a retype cleared, has no boundaries to offer. Then the section falls back to a single "Split at the playhead" button, disabled (with the reason) unless the playhead is strictly inside the caption; Rust cuts that one at the nearest space in the text.

### Merge

Disabled on the last caption, with the reason both on the button's `title` and in the hint, rather than letting a press do nothing. When it is live the hint names where the joined caption will end, so the result is knowable before the press. The earlier caption's id survives the merge (Rust's rule), so the selection does not jump.

### Word timings

The Text section's value slot says `N words` or `typed`, and its hint says plainly that retyping the words drops the highlight timings for that caption, and why. See `captionEdit.md` for the full argument for dropping rather than redistributing.

### Look

`InspectorShell kind="caption"` gives it `--insp-accent: var(--e-wave)` (inspectors.css) - the editor's one audio hue, because a caption is what the audio said. New tokens: none.

### Used by

- `src/editor/shell/PropertiesSlot.tsx` - the `hit.kind === "caption"` branch.
