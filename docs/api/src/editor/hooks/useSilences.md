# src/editor/hooks/useSilences.ts

The transport's Remove silences, the editor half of `src-tauri/src/export/pipeline/silence.rs`.

## newSilences

```ts
export function newSilences(spans: [number, number][], cuts: { start_ms: number; end_ms: number }[]): [number, number][]
```

The detected spans not already inside an existing cut, so a second run does not report again what the first one removed.

## silenceToast

```ts
export function silenceToast(spans: [number, number][]): string
```

`Removed 2 silences, 2.4 s`, `Removed 1 silence, 0.7 s`, or `No silences found`.

## useSilences

```ts
export function useSilences(folder: string, docRef: RefObject<EditDoc | null>, applyOp: (op: EditOp) => Promise<EditDoc | null>, toast: (msg: string) => void): () => Promise<void>
```

The callback the button calls: `detectSilences(folder)`, `newSilences` against the current doc's cuts, ONE `add_cuts` (one undo step) when anything is left, then the toast. A scan failure is a toast too, never a silent nothing.
