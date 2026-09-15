# src/editor/hooks/stage/useExactFrame.ts

The stage's exact-frame path. Owner ruling (2026-09-14): the export renders effects correctly and is the reference; the preview must match it 1:1, so parity fixes go into the preview. The live canvas is a TypeScript re-composition (`previewCanvas.ts`, the FX overlay, the cursor sprites) and cannot be bit-identical to the Rust renderer; what CAN be identical is every frame looked at while the playhead rests. Whenever playback is paused or a scrub has settled, this hook asks the Rust renderer for the export's own composited frame at that instant (`preview_frame`, the warm-cached `FrameRenderer` - the export's compositor, effects and cursor) and `useCompositeLoop` draws it over the live composite. Playing resumes the live composite.

## ExactFrame

```ts
export interface ExactFrame { key: string; img: HTMLImageElement }
```

One fetched frame: the key it was made for (`exactKey`) and the decoded image.

## SETTLE_MS

```ts
export const SETTLE_MS: number   // 160
```

How long the playhead (or the doc) must hold still before a frame is asked for: long enough that a scrub does not fire a request per pixel, short enough that a pause feels answered. The live composite shows in the meantime.

## exactKey

```ts
export function exactKey(timeMs: number, editGen: number): string
```

`"<rounded instant>|<edit generation>"`. A frame from another instant or an older doc never matches, so it is never drawn stale. Tests: `useExactFrame.test.ts`.

## wantsExact

```ts
export function wantsExact(playing: boolean, draft: boolean, folder: string, key: string, held: ExactFrame | null): boolean
```

Pure gate: paused, a folder to ask about, no unsaved draft the file would not know about, and not already holding the frame for this key.

## useExactFrame

```ts
export function useExactFrame({ folder, playing, timeMs, draft, dirtyRef, deps }: {
  folder: string; playing: boolean; timeMs: number; draft: boolean; dirtyRef: RefObject<boolean>; deps: unknown[];
}): { exactRef: RefObject<ExactFrame | null>; editGenRef: RefObject<number> }
```

### Inputs

- `folder` - the project (`ClassicShell` passes `p.folder` to `Stage`). `""` disables the path.
- `playing` / `timeMs` - the transport. The request keys on `timeMs` in CLIP time, the same clock `preview_frame` takes.
- `draft` - `moveMode || arranging`: while a Move or Arrange draft is live nothing is requested, since the file the renderer reads does not know the draft and a frame without it would contradict the drag.
- `dirtyRef` - the loop's paused-redraw flag; set once a frame has decoded so the loop repaints with it.
- `deps` - everything that changes what a frame looks like except the playhead (`Stage` passes its draw deps minus `timeMs`/`playing`, plus `moveMode` and `bg`). Any change bumps the edit generation, which retires the held frame (its key no longer matches) and, after `SETTLE_MS`, requests a fresh one.

### Returns

`exactRef` (the held frame, or null) and `editGenRef` (the current generation), both for `useCompositeLoop`, which draws `exactRef.current.img` over the composite on a paused tick whose `exactKey(t, editGenRef.current)` equals the held key.

### Behavior

- One effect on `[folder, playing, timeMs, draft, editGen]`: when `wantsExact`, start a `SETTLE_MS` timer, then `previewFrame(folder, timeMs)`, decode the data URL into an `Image`, and on load store `{ key, img }` and mark the canvas dirty. The effect's cleanup sets a `live` latch false and clears the timer, so a newer instant, a newer doc, a play or a draft supersedes an in-flight request and its answer is dropped.
- The edit generation is a `useMemo` over `deps` that increments a counter, so it is computed during render and the effect re-runs on the next commit.
- A request failure leaves the live composite in place; the next settle asks again.
- Cost per settle is one Rust round trip: a seek-decode of the raw frame, the composite, a JPEG encode (`preview::jpeg_encode`) and the IPC. The live composite is on screen the whole time, so the exact frame replaces it a moment after the playhead rests.

### Used by

- `src/editor/stage/Stage.tsx` - `useExactFrame({ folder, playing, timeMs, draft: moveMode || arranging, dirtyRef, deps: [...] })`, its refs handed to `useCompositeLoop`.
