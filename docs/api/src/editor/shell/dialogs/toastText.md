# src/editor/shell/dialogs/toastText.ts

Display-length capping for the `Toast` pill.

## truncateToastText

```ts
export function truncateToastText(text: string, max = 120): string
```

Caps how much of a message the `Toast` pill (`Toast.tsx`) renders inline. The pill was built for `"Undid"`/`"Redid"` (5 characters); Task 11 reuses it for Rust's raw `export-warning` strings, which can run long (they splice in a raw decode-error message, see `mod.rs`'s `webcam_warning`). Truncates on a word boundary where one exists past 60% of `max`, so it doesn't cut mid-word; the SOURCE string is never altered - this only shapes what one transient pill shows (the full text is still whatever the caller has, e.g. in a log). Text at or under `max` passes through unchanged. Unit-tested (`toastText.test.ts`).

### Used by

`useUndoToast`'s `push` (`Toast.tsx`) - applied to every pushed message, not just export warnings, so any future caller of `push` gets the same protection automatically.
