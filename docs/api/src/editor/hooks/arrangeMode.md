# src/editor/hooks/arrangeMode.ts

The pure decision behind stage arrange mode - kept out of `useArrangeMode` so every entry/exit rule is unit-testable with plain object literals (`arrangeMode.test.ts`), the same split `keymap.ts` uses against `useEditorKeymap`.

## ArrangeMode

```ts
export interface ArrangeMode { on: boolean; segId: string | null }
```

Whether the stage is showing the panel frames, and which layout segment they belong to. The two are deliberately independent: `segId` SURVIVES an Escape, which is what lets `LayoutInspector`'s "Arrange on stage" button put the user straight back into the mode they just left without re-selecting the pill.

## ArrangeEvent

```ts
export type ArrangeEvent =
  | { kind: "select"; segId: string | null }
  | { kind: "arrange" }
  | { kind: "escape" }
  | { kind: "gone" };
```

`select` carries the LAYOUT segment the editor selection landed on, or `null` for a deselect or any other kind of selection (a zoom, an effect, a camera keyframe). `gone` is the arranged segment disappearing - deleted, or a doc reload without it.

## NO_ARRANGE

```ts
export const NO_ARRANGE: ArrangeMode = { on: false, segId: null };
```

The initial state, and the state both `select: null` and `gone` return - returned by identity so an event that changes nothing cannot cause a re-render.

## nextArrangeMode

```ts
export function nextArrangeMode(prev: ArrangeMode, ev: ArrangeEvent): ArrangeMode
```

| event | result |
|---|---|
| `select` with an id | `{ on: true, segId }` - selecting a layout pill IS the entry gesture (binding UX), including re-selecting the same segment after an Escape |
| `select` with `null` | `NO_ARRANGE` - exits AND forgets the segment |
| `arrange` | on, keeping the remembered `segId`; inert when nothing is remembered |
| `escape` | off, REMEMBERING `segId`; a no-op (same object back) when already off |
| `gone` | `NO_ARRANGE` |

## arrangeSeekMs

```ts
export function arrangeSeekMs(seg: LayoutSeg, timeMs: number): number | null
```

Where the playhead has to go for arrange mode's frames to be showing something real: `null` when it is already inside `[start_ms, end_ms)`, else `start_ms + transition_ms` - the first instant the layout is SETTLED rather than mid-fade - clamped to stay inside the span (a transition longer than the segment lands on `end_ms - 1`, never before `start_ms`).

Not cosmetic. `layoutAt` only selects a segment while the playhead is inside its span, so from outside it the live drag draft has no effect on the composite at all. Called once per entry by `useArrangeMode`.
