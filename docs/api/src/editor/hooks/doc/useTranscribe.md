# src/editor/hooks/doc/useTranscribe.ts

The Captions panel's whole backend side: the model table, a model download, and a transcription run, plus the six events either of them reports through. Nothing else in the editor listens for an `asr-*` event, so this hook is the only place that knows those names.

## Transcribe

```ts
export interface Transcribe {
  models: WhisperModelDto[];
  refreshModels: () => void;
  phase: Phase;
  pct: number;
  error: string | null;
  download: (id: string) => void;
  transcribe: () => void;
}
```

`pct` is 0..100 for whatever `phase` names and is meaningless at rest. `error` is the backend's message VERBATIM - the panel never re-words it, because the useful ones ("This recording has no audio to transcribe.", "Base (English) is not downloaded yet.") already say exactly what to do.

## useTranscribe

```ts
export function useTranscribe(folder: string, onDone?: () => void): Transcribe
```

Subscribes to all six events once on mount, BEFORE any button exists to start a run - the `useRecordingFlow` rule: a listener registered inside a click handler can miss the first event the backend emits, and `download_whisper_model` starts emitting before its own promise resolves.

| event | effect |
| --- | --- |
| `asr-download-progress` | phase `downloading`, `pct` from `done/total` |
| `asr-download-done` | phase `idle`, and `refreshModels()` so the button flips from Download to Transcribe on its own |
| `asr-download-error` | phase `idle`, message kept |
| `asr-progress` | phase `decoding` or `transcribing` from the payload's `phase`, `pct` clamped |
| `asr-done` | phase `idle`, then `onDone()` |
| `asr-error` | phase `idle`, message kept |

`onDone` is held in a ref rather than named in the effect's dependency array: the panel re-creates the callback on every render, and a dependency on it would tear the six listeners down and re-subscribe them mid-transcription. It is what `CaptionsPanel` passes `reloadDoc` to - transcription writes `edit.json` on the backend under the doc lock (ADDED-8), so the doc has to be re-read rather than patched from an op.

`download` and `transcribe` set the phase OPTIMISTICALLY (`downloading` / `decoding`) before the IPC call, which closes the window between the press and the first event where the button would still be pressable. Both clear `error` first, so a previous failure never sits under a new run, and both fall back to `idle` with the rejection's text if the invoke itself rejects.

### Used by

- `src/editor/panels/captions/CaptionsPanel.tsx` - the only caller.
