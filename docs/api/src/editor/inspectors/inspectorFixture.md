# src/editor/inspectors/inspectorFixture.tsx

The shared fixture for the inspector DOM tests. `inspectorShape.test.tsx` (the one shape every inspector is built from) and `inspectorSections.test.tsx` (the Zoom hero, the grouped timing row, the Motion section) both mount against it, so the two files cannot drift on what a zoom, a spotlight or a layout segment looks like. It is not a `.test.tsx` file and runs no assertions of its own beyond `parsed`-style guards - it only builds and queries.

## useInspectorDom

```ts
export function useInspectorDom(): void
```

Wires the per-test React root: a `beforeEach` that sets `IS_REACT_ACT_ENVIRONMENT`, empties `ops` and mounts a fresh `createRoot` on a fresh container, and an `afterEach` that unmounts and removes it. Called once at a test file's module scope, before its `describe`s - vitest registers the hooks against the file that imported it, so the two test files get their own isolated root each.

## show

```tsx
export const show = (node: ReactNode) => void
```

Renders `node` into the fixture's root inside `act`.

## q

```ts
export const q = <T extends Element>(sel: string) => T | null
```

`container.querySelector` against the current root. `qa` is the `querySelectorAll` array form. Every query helper below is built from these two, so no test reaches for `document` and picks up a leaked node from an earlier case.

## qa

```ts
export const qa = <T extends Element>(sel: string) => T[]
```

## zoomAt

```tsx
export const zoomAt = (z: Partial<Zoom> = {}) => void
```

Renders `ZoomInspector` over the `ZOOM` fixture with `z` merged in - the one-liner most zoom cases start from (`zoomAt({ scale: 2.8 })`, `zoomAt({ zoom_in_ms: 0 })`).

## layoutAt

```tsx
export const layoutAt = () => void
```

Renders `LayoutInspector` over `SEG` with the `PRESETS` stand-in, so its Composition row has four presets to draw thumbnails for.

## ops

```ts
export const ops: EditOp[]
```

Every op `apply` received since the current test started (`useInspectorDom` empties it in `beforeEach`). `apply` is the `onApply` every inspector is handed; it records and resolves `null`, so nothing in these tests touches the backend.

## apply

```ts
export const apply = async (op: EditOp) => null
```

## noop

```ts
export const noop = () => {}
```

## ZOOM

```ts
export const ZOOM: Zoom
```

A 1.00s-to-3.60s cursor-target zoom at 2.2x with 350/450ms ramps - the span `inspectorShape.test.tsx` asserts the header range from, and the four values the grouped Timing row reads.

## FX

```ts
export const FX: EffectRegion
```

## SEG

```ts
export const SEG: LayoutSeg
```

## MOVE

```ts
export const MOVE: CameraMove
```

## CUT

```ts
export const CUT: Cut
```

## SPEED

```ts
export const SPEED: Speed
```

## SETTINGS

```ts
export const SETTINGS: Settings
```

Only the `clickfx` block `EffectInspector` reads, cast through `unknown` - the rest of `Settings` is never touched by an inspector.

## PRESETS

```ts
export const PRESETS: LayoutPresets
```

Five layout presets built from one `rect`/`preset` pair, each with its own arrangement, plus an empty `segs` and an `inset_w`. Enough for `LayoutInspector` to resolve panels and draw four thumbnails; not a faithful copy of what the backend sends.

## sections

```ts
export const sections = () => (string | null)[]
```

Every `.e-isec-head h3` in DOM order - the assertion that pins each inspector's section list.

## segs

```ts
export const segs = (group: string) => HTMLButtonElement[]
```

The `SegRow` buttons inside the group with that `aria-label`; `active(group)` is the same list filtered to the lit one.

## active

```ts
export const active = (group: string) => (string | null)[]
```

## motionSegs

```ts
export const motionSegs = () => HTMLButtonElement[]
```

The Motion section's preset row, which is a `Segmented` (`.e-segment`) rather than a `SegRow`; `motionActive()` is its lit entry.

## motionActive

```ts
export const motionActive = () => (string | null)[]
```

## range

```ts
export const range = () => string | null | undefined
```

The header's `.e-ihead-range` line.

## del

```ts
export const del = () => HTMLButtonElement | null
```

The header's Delete button.
