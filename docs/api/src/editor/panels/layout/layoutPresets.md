# src/editor/panels/layout/layoutPresets.ts

The Layouts panel's arithmetic with no React in it: which layout the playhead is sitting in, whether a typed name may be saved, and the list edits. Every list helper takes and returns a **`Settings`-shaped value** (anything with a `layout_presets` field) and returns a new one, so the panel's own job reduces to `setSettings(next)` - a read-modify-write that keeps every other global field.

## BUILTIN_PRESET

```ts
export const BUILTIN_PRESET: LayoutPreset
```

The built-in row - `{ id: "default", name: "Default", appearance: DEFAULT_APPEARANCE }`. It is **not** in `Settings.layout_presets`: it is prepended for display only, which is exactly why `removePreset` can never delete it and why no config migration was needed to introduce it. It participates in name validation (so "Default" is a taken name) and is always first, so a user who has wandered can get back without having saved anything first.

## MAX_PRESET_NAME

```ts
export const MAX_PRESET_NAME: number
```

40. Longer and a row's name would have to ellipsise at 320px, which is the truncated-name bug the panel pass fixed elsewhere - so it is refused at the source instead of hidden by CSS.

## layoutAtPlayhead

```ts
export function layoutAtPlayhead(segs: LayoutSeg[], timeMs: number): ModeKey
```

Which layout the playhead is inside, for the panel's opening pick. Segments are half-open (`start_ms <= t < end_ms`), so a segment's own end belongs to whatever follows it, and the **last** covering segment wins (later segments draw over earlier ones). Everything else reads as `"screen"`: no segments at all, a gap between two, a time before the first, or a `layout` string this build does not know (`LayoutSeg.layout` is a free `string` on the wire, and only the five ids in `LAYOUT_KEYS` name an appearance block).

Pure and read-only - it chooses what to SHOW, never what to write.

## presetNameError

```ts
export function presetNameError(name: string, list: LayoutPreset[], exceptId?: string): string | null
```

Why this name cannot be saved, or `null` when it can. Trimmed first; empty, over-long and duplicate are the three refusals. Duplicates are compared case- and whitespace-insensitively and **against `BUILTIN_PRESET` too**. `exceptId` is the row being renamed, so renaming a look to its own current name is allowed rather than reported as a clash with itself.

Returns the sentence the panel shows, not a code: there is exactly one caller and three messages, and keeping them here is what lets the save field and the rename field enforce the same rule with the same words.

## nextPresetId

```ts
export function nextPresetId(list: LayoutPreset[]): string
```

The next free `lp<n>`. Derived from the list rather than from a counter or `Date.now()`, so the same list always yields the same next id - which is what makes `addPreset` assertable in a test without stubbing a clock.

## addPreset

```ts
export function addPreset<S extends { layout_presets: LayoutPreset[] }>(settings: S, name: string, appearance: AppearanceSettings): S
```

Appends `appearance` as a new saved look, under the trimmed name. Returns the settings **unchanged (same reference)** when `presetNameError` refuses the name, so a caller that forgot to check cannot write a duplicate; the panel checks in the name field and this is the backstop.

## renamePreset

```ts
export function renamePreset<S extends { layout_presets: LayoutPreset[] }>(settings: S, id: string, name: string): S
```

Renames one look in place - same id, same position in the list, same appearance. Unchanged when the new name is refused or `id` names nothing.

## removePreset

```ts
export function removePreset<S extends { layout_presets: LayoutPreset[] }>(settings: S, id: string): S
```

Drops one saved look. The built-in row is not in the list, so passing its id is a no-op rather than a special case.

## resetLayout

```ts
export function resetLayout(appearance: AppearanceSettings, mode: ModeKey): AppearanceSettings
```

One layout back to how it ships (`DEFAULT_APPEARANCE[mode]`), leaving the other four at the exact objects they were. It replaces the old `resetCameraAppearance` (`hud/preferences/appearanceFields.ts`), which reset eight named fields of the `screen` mode only - the generalization the Layouts panel needed, and it closes the same class of bug that helper was written for: a reset that writes `DEFAULT_APPEARANCE` wholesale silently resets all five.

### Used by

- `src/editor/panels/layout/LayoutsPanel.tsx` - the picker's default, the reset button, and the four preset write callbacks.
- `src/editor/panels/layout/LayoutPresetList.tsx` - `BUILTIN_PRESET`, `MAX_PRESET_NAME` and `presetNameError` (the name field's `check`).
