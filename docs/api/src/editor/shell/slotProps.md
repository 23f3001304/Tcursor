# src/editor/shell/slotProps.ts

The one bundle of editor state that travels from `Editor.tsx` down to whichever areas happen to be on screen.

## SlotProps

**Time remap.** Two fields from `useTimeMap` in `Editor.tsx`: `map: TimeMap` (the clip-to-output clock map built from the doc's trim, cuts and speed spans) and `outDoc: EditDoc` (the doc's regions on the output clock). `StageSlot` hands the stage `outDoc`'s regions and `map`, and gives the transport `outOf(map, timeMs)` and `outDurMs(map)`; the timeline keeps `doc` (clip time).

**Range selection (T7).** `range: [number, number] | null` and `setRange`, a plain `useState` pair in `Editor.tsx`. It is in the bundle for the same reason `sel` is: two different areas need it and neither owns it. The TIMELINE edits it (the ruler's Shift+drag, `timeline/useRangeSelect.ts`) and draws it (`RangeOverlay`); the TRANSPORT acts on it (`TransportTools`' Cut and Speed) and clears it afterwards. In clip ms, like every other time in the bundle except the transport's own readout.

```ts
export interface SlotProps { /* ~70 fields - see the source */ }
```

Everything the four editor types need between them: the doc, the selection, the panel tab, the playhead and transport state, the preview data `useEditorData` fetched, the callbacks `useEditorCallbacks` / `useTimelineActions` / `useTrimActions` built, and the AI director's live state.

**The panel tab is nullable (2026-09-14).** `tab: Tab | null` and `setTab: Dispatch<SetStateAction<Tab | null>>`, where `null` means the panel column is collapsed - a real resting state, not an error one (`panelState.md`). `onTab` is unchanged in shape but changed in meaning: it is no longer "show this tab" but "the user pressed this tab", and `ClassicShell` resolves that through `nextTab` so a second press on the open tab closes.

### Why a bundle and not props

Before M1a, `Editor.tsx` composed `Stage`, `Transport`, `Timeline` and `EditorPanels` itself, so each of those took its own props at the one call site. After M1a nobody knows at compile time which editors exist or how many - the tree decides - so the values have to reach `EditorSlot` and be spread out there. Passing them as one object is what let `Editor.tsx` drop back under its line cap (the JSX it lost was ~45 lines; the literal that replaced it is ~12).

Field names deliberately match `Editor.tsx`'s own locals, so the hand-off is a shorthand object literal rather than seventy re-typed `name={name}` attributes.

### What is NOT in it

Anything derivable from `doc`: the aspect, the zooms, layout segments, camera moves, effects, the cursor/clickfx/zoom settings blocks, the AI model name. The slots read those off `p.doc` at the point of use. Adding them as fields would have meant two ways to ask the same question and a stale-copy bug waiting to happen.

### Identity and memo

`Editor.tsx` rebuilds this object every render, which is fine: `EditorSlot` spreads it back into individual props, and `Stage` / `Transport` / `Timeline` / `EditorPanels` are all `React.memo`'d on those individual values. The render-hygiene work that made those callbacks stable (see `Editor.md`, "render hygiene") is what still does the skipping; the bundle is only a transport.

## ShellProps

```ts
```

What `Editor.tsx` actually passes. It differs from `SlotProps` at both ends:

- `onTab` is one of two fields the shell supplies itself: only the shell knows whether the pressed tab is the one already showing, which is what decides between opening and collapsing (`nextTab`, `panelState.ts`).
- `modalOpen` is added on rather than folded into `SlotProps`, so the editors never see it. It is true while any dialog, overlay or the AI director's scrim owns the screen - the same boolean `Editor.tsx` already computes for `useEditorKeymap` - and the shell's own keyboard (`Ctrl+Space` to maximize, A3) goes inert behind it exactly as every shortcut in `keymap.ts` does. Only the shell has a keymap of its own, so only the shell is given the flag.

`onDetectSilences: () => void` (from `useSilences` in `Editor.tsx`) is the transport's Remove silences.

Since the classic layout came back, `ClassicShell` is the one consumer: it hands `Stage`, `Transport`, `Timeline`, `EditorPanels` and `PropertiesSlot` their props from this bundle. `ShellProps` omits only `onTab` (the shell derives it: it also drops the selection).
