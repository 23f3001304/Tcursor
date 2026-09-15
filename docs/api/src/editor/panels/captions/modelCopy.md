# src/editor/panels/captions/modelCopy.ts

Every word the Captions panel's transcribe card says, and the one state machine behind its primary button. Pure and React-free on purpose: the copy is pinned by `modelCopy.test.ts` rather than read off a screenshot, and `TranscribeCard.tsx` renders what this file decides without deciding anything itself.

## Phase

```ts
export type Phase = "idle" | "downloading" | "decoding" | "transcribing"
```

Where a run is. `"downloading"` is the MODEL fetch (`asr-download-progress`), not the transcription; `"decoding"` and `"transcribing"` are the two phases `asr-progress` reports (`decode` then `transcribe`). `useTranscribe` owns the value; this file only reads it.

## Action

```ts
export interface Action { kind: "download" | "transcribe" | "busy" | "pick"; label: string; disabled: boolean }
```

What the card's one primary button IS right now. `kind` is what a press does (the card routes on it), `label` is the whole word on the button, and `disabled` is the truth about whether it can be pressed - so no caller re-derives it from the phase and gets a different answer.

## sizeWarning

```ts
export const sizeWarning: (bytes: number) => string
```

`"141 MB download, once"`. MB is 1024-based so the number matches what the OS file browser will show once the file is on disk, and it is rounded because nobody needs the bytes. "once" is the whole point of stating it: the cost is paid one time per model, not per transcription.

## actionFor

```ts
export function actionFor(model: WhisperModelDto | null, phase: Phase): Action
```

The primary button for this model in this phase, as a four-branch table:

| state | kind | label |
| --- | --- | --- |
| phase `downloading` | `busy` | Downloading |
| phase `decoding` | `busy` | Reading the audio |
| phase `transcribing` | `busy` | Transcribing |
| no model picked | `pick` | Pick a model |
| model not on disk | `download` | Download model |
| model on disk | `transcribe` | Transcribe audio |

A busy phase wins over everything else, including "nothing is picked": offering a second run while one is in flight is the one thing the card must never do, and a bar whose caption went blank mid-run would read as a hang.

## languageOptions

```ts
export function languageOptions(model: WhisperModelDto | null): { value: string; label: string; disabled: boolean; title?: string }[]
```

The two languages, with Auto detect marked `disabled` (and carrying the reason as its `title`) whenever the picked model is English-only. Rust's `asr::models::resolve_model` refuses that pair outright, so the card must not offer it; `TranscribeCard` renders the enabled options as a `Segmented` and puts the disabled one's `title` on an `.e-hintline` under it, because neither `Picker` nor `Segmented` can disable a single option.

## modelOptionLabel

```ts
export const modelOptionLabel: (m: WhisperModelDto) => string
```

A model picker row: the label, plus `sizeWarning(bytes)` in parentheses while the file is not on disk yet. The cost sits on the option the user is about to choose rather than somewhere else on the card.

### Used by

- `src/editor/panels/captions/TranscribeCard.tsx` - the whole card.
- `src/editor/hooks/doc/useTranscribe.ts` - imports `Phase` (defined here so there is one definition).
