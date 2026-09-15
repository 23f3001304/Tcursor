# src/editor/shell/useDensity.ts

The live density for the editor's viewport (`density.md`), as a module store with one `resize`
listener - the same shape `stage/transport/viewMode.ts` uses, and for the same reason.

**Why a store and not props or state.** The three readers have no shared parent below `Editor`:
`Editor` writes the custom properties onto `.editor`, `ClassicShell` animates the two columns to
their current widths, and `Timeline` sizes its lane-label gutter rows from the same lane heights.
Threading a `Density` through `ShellProps` would put a layout constant into a props bag that is
otherwise all document state, and would make every one of the shell's memo boundaries depend on it.
Lift it to `Editor` state the day something needs a density that is not the window's.

**Why the snapshot re-measures on every read.** `measure()` runs on each `getSnapshot` and returns
the cached object by IDENTITY when nothing crossed a step (`sameDensity`), which satisfies
`useSyncExternalStore`'s caching rule *and* closes the one race the listener alone would lose: this
module is imported while the HUD's 980 x 132 window is still up, and `App.openEditor` resizes the
window before React mounts `Editor` - so a snapshot taken only at module load would report the HUD's
size for the editor's first frames.

**Lifetime.** The listener is attached on the first subscription and never removed. The editor is a
full-window view; when it unmounts the app is on its way back to the HUD, whose own window resize
just recomputes a value nobody is reading. The server snapshot is the same `current` object, so
nothing here depends on a `window` existing at import time.

## useDensity

```ts
export function useDensity(): Density
```

Subscribes the calling component to the store. Re-renders only when a resize actually crosses a
step, so dragging a window edge inside one band costs nothing.

## useDensityVars

```ts
export function useDensityVars(): CSSProperties
```

`useDensity` as the inline style `Editor` puts on the `.editor` element - the one place the whole
ladder enters CSS. Memoized on the density object, so a playhead tick never hands React a fresh
style object to diff.
