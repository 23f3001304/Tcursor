# src/editor/panels/captions/CaptionsPanel.tsx

The Captions rail tab: turn what was said into a track, style it, and read it back. Replaces the placeholder `EditorPanels` rendered under `tab === "captions"` since M5 T1.

## CaptionsPanel

```tsx
export function CaptionsPanel({ folder, doc, applyOp, saveDocSettings, reloadDoc, timeMs, sel, onSeek, onSel, onClose }: {
  folder: string; doc: EditDoc;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  saveDocSettings: (s: EditDoc["settings"]) => void;
  reloadDoc: () => void;
  timeMs: number; sel: string | null;
  onSeek: (ms: number) => void; onSel: (id: string | null) => void; onClose: () => void;
}): JSX.Element
```

Three blocks, in the order the work happens:

1. `TranscribeCard` - a model onto the machine, then the run.
2. `CaptionStyleControls` - how the result looks, written to `doc.settings.captions`. It is handed `doc.settings.ui.accent` as well, for the highlight field's Accent swatch (see `CaptionColorFields.md`).
3. The transcript (`CaptionList`) plus one text button, "Clear all captions", behind a `ConfirmDialog`.

The transcript's section heading is the count - "87 captions", or "Transcript" while there are none. A number is what someone wants from that line, and it was already being spent on the Clear button's label, which now just says what it does.

The panel root carries `.e-cappanel` on top of `.e-panel`, which makes it a flex column so the transcript can take the height the first two blocks leave and scroll inside itself. This is the one panel that breaks panels.css rule 6 (a panel fits its slot without scrolling), and it breaks it on purpose: the length of a transcript is the recording's, not the design's.

### What this panel deliberately does NOT do

Edit a caption. A caption is a region on the timeline like a zoom or a cut, so it is edited where every other region is edited: select it and the properties sidebar shows `CaptionInspector`. Two places to change one caption's text would be one place too many, and the transcript's job is to find the caption, not to be a text editor.

### Why `reloadDoc` and not an op

Transcription is the one edit the frontend does not make itself. `asr::commands::transcribe_project` applies `EditOp::SetCaptions` to `edit.json` under `edit::lock::doc_lock` and emits `asr-done` (ADDED-8: nothing ships a caption array back over IPC), so the panel asks for the doc to be re-read rather than applying an op it does not have the captions for. `Editor.tsx`'s `reloadDoc` snapshots the pre-write doc first, so a transcription is one undo step like every other edit.

### Reset

The header's Reset writes `DEFAULT_CAPTION_STYLE` over the four look fields and keeps `model` and `language` - see `CaptionStyleControls.md`.

### Clearing

`clear_captions` is behind a confirm because it is the one caption action that is not recoverable by repeating it: transcribing again re-creates the lines from the audio, but any text edited by hand is gone. The confirm's body says exactly that. It deselects first, so the sidebar cannot be left holding a caption that no longer exists.

### Used by

- `src/editor/EditorPanels.tsx` - the `tab === "captions"` branch.
