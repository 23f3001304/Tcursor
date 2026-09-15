# src/shared/ipc/asr.ts

The speech-to-text surface of the IPC client: the Whisper model catalogue (`whisperModels`, `downloadWhisperModel`) and the transcribe run (`transcribeProject`) with its two progress events. Split back out of `ipc.ts`; `ipc.ts` re-exports it, so `from "../shared/ipc"` keeps working everywhere.

## WhisperModelDto

```ts
export interface WhisperModelDto { id: string; label: string; bytes: number; installed: boolean; multilingual: boolean }
```

One Whisper model the app knows about - mirrors Rust `asr::commands::ModelDto`.

`bytes` is the exact file size, shown as the size warning before a download starts: `base.en` is about 141 MiB and `small.en` about 465 MiB, and a user on a metered connection has a right to know that before the bar appears. `multilingual` gates the `language: "auto"` option; `installed` is resolved from the file on disk at call time, not cached.

## whisperModels

```ts
export const whisperModels: () => Promise<WhisperModelDto[]>
```

Every model in the Rust table, with `installed` resolved from the file on disk right now. Cheap enough to call whenever the Captions panel opens or a download finishes - re-fetching is the intended way to refresh `installed`, rather than patching local state from the event.

## downloadWhisperModel

```ts
export const downloadWhisperModel: (id: string) => Promise<void>
```

Starts a download on the backend and returns at once. The promise resolving means "the backend thread was spawned", NOT "the model is installed".

Progress arrives as `asr-download-progress` events and ends with exactly one `asr-download-done` (payload: the id) or `asr-download-error` (payload: a message meant to be shown verbatim). Asking twice for an id that is already downloading is a no-op on the backend, so a double-click is harmless and the caller does not need its own guard.

## transcribeProject

```ts
export const transcribeProject: (folder: string) => Promise<void>
```

Starts transcription on the backend and returns at once. Progress arrives as `asr-progress` and ends with exactly one `asr-done` (the caption count) or `asr-error`. The captions are written into `edit.json` by Rust itself, under `edit::lock::doc_lock` (ADDED-8: nothing ships a caption array back over IPC), so the caller re-fetches the doc rather than patching it - `useTranscribe`'s `onDone`, which `CaptionsPanel` wires to `Editor.tsx`'s `reloadDoc`.

Asking twice for a folder already in flight is a no-op on the backend, so a double-click is harmless.

## AsrProgress

```ts
export interface AsrProgress { phase: "decode" | "transcribe"; pct: number }
```

The `asr-progress` payload. `decode` is the ffmpeg pass that turns the recording's own WAV into 16 kHz mono f32; `transcribe` is whisper.cpp itself, reporting through its progress callback. `useTranscribe` maps the two onto its `"decoding"` / `"transcribing"` phases, which is what the transcribe card's button label and bar read.

## AsrDownloadProgress

```ts
export interface AsrDownloadProgress { id: string; done: number; total: number }
```

The `asr-download-progress` payload - mirrors Rust `asr::commands::DownloadProgress`. `total` is the table's byte count rather than a `Content-Length`, so a resumed download's bar starts where it left off. `id` is present because several rows can be shown at once and a progress event must be routed to the right one.
