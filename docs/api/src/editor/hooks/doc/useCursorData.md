# src/editor/hooks/doc/useCursorData.ts

The recording's cursor data: the kind track, whether the OS cursor is already baked into the video, the captured cursor layer, and the selected pack's decoded sprites. Split out of `useEditorData.ts` - these four fetches are one subsystem with two fetch keys of their own, and together they decide what the Stage draws for the `System` cursor style.

## useCursorData

```ts
export function useCursorData(folder: string, doc: EditDoc | null): {
  cursorSpr: CursorPackDto | null; cursorKnd: CursorKindSample[];
  cursorLyr: CursorLayerDto | null; osCursor: boolean;
}
```

**Timeline-independent tracks (`[folder]`).** `clickTrack` -> `clicks`, `cursorKinds` -> `cursorKnd`, `cursorLayer` -> `cursorLyr` and `osCursorInVideo` -> `osCursor` are immutable per recording, so they fetch once per folder rather than on every `rev` bump (refetching them on every edit was part of the earlier add-effect lag). `osCursor` starts at `true` - "the video already has the OS cursor", the answer that preserves today's behavior while the fetch is in flight - and tells `Stage`/`CursorPanel` whether the `System` cursor style has to be re-created from the recorded path. `cursorLyr` starts at `null` ("no captured layer"), the matching safe answer, and is what lets `Stage` composite the REAL recorded cursor for `System` instead of a plain arrow.

**Cursor sprites (`[folder, doc?.settings.cursor.pack]`).** `cursorSprites` -> `cursorSpr` refetches only when the selected pack changes (picking a different pack, or importing one, in `CursorPanel`) - not on generic `rev` bumps, so unrelated edits don't re-decode sprites.

Both effects use the standard `live` cleanup-token guard - see `useEditorData.md`.
