# src/editor/hooks/useDocSettings.ts

Settings-only doc writes, split out of `Editor` - bulk `settings` patches (background, cursor, camera, captions, audio, the AI model) as opposed to `applyOp`'s per-`EditOp` mutations.

## useDocSettings

```ts
export function useDocSettings(
  folder: string,
  doc: EditDoc | null,
  setDoc: (d: EditDoc) => void,
  record: (current: EditDoc) => void,
  setRev: (fn: (r: number) => number) => void,
): { saveDocSettings: (s: EditDoc["settings"]) => Promise<void>; onAutoModel: (v: string) => void }
```

### Inputs

- `folder: string` - project directory, passed to `saveEdit`.
- `doc: EditDoc | null` - the current doc; both returned writers patch its `settings` field.
- `setDoc: (d: EditDoc) => void` - swaps in the patched doc.
- `record: (current: EditDoc) => void` - `useEditHistory`'s undo-stack push; called by `saveDocSettings`, NOT by `onAutoModel`.
- `setRev: (fn: (r: number) => number) => void` - bumped after every write so `rev`-keyed preview data refetches.

### Returns

- `saveDocSettings: (s: EditDoc["settings"]) => Promise<void>` - `record(doc)` then write. The normal path for every settings-editing panel; a manual pick in `AiPanel`'s Engine dropdown goes through this too, via `EditorPanels`.
- `onAutoModel: (v: string) => void` - write WITHOUT `record()` - a quiet variant. The only caller is `AiPanel`'s mount-time "default `ai_model` to a real installed Ollama model" effect: since that effect fires from mounting/model-list-resolving, not a user edit, routing it through `saveDocSettings` would push a phantom undo step and an unasked-for disk write the instant the AI panel opens. Memoized on `doc` (`useCallback`, not recreated on every render - e.g. every `timeMs` tick during playback) so `AiPanel`'s effect, keyed on this callback's identity, doesn't needlessly re-fire while the doc hasn't actually changed.

### Used by

`Editor` (`src/editor/Editor.tsx`) - `saveDocSettings` is passed to `EditorPanels` (and every settings panel beneath it); `onAutoModel` is passed through to `AiPanel` alone.
