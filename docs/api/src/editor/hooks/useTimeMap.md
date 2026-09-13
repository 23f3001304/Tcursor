# src/editor/hooks/useTimeMap.ts

The editor's one place where the time remap's two clocks meet (`docs/superpowers/specs/2026-09-13-time-remap-design.md`).

## useTimeMap

```ts
export function useTimeMap(doc: EditDoc | null, fullDurMs: number): { map: TimeMap; outDoc: EditDoc | null }
```

The clip-to-output clock map built from the doc's trim, cuts and speed spans (`lib/remap.ts`) and the doc's regions moved onto the output clock (`lib/remapDoc.ts`), memoised on `[doc, fullDurMs]` so it is recomputed once per doc change and never per tick. `outDoc` is what the stage evaluates (zooms, layouts, effects, camera moves in the time the viewer sees); the timeline keeps `doc` itself, whose pills sit on clip time and never move when a cut is added. `fullDurMs` is the raw clip's length (`Editor.tsx`'s `dur`). With no doc yet: an identity map over `fullDurMs` and no `outDoc`. Both land in `SlotProps` as `map` and `outDoc`.
