# src/editor/panels/CursorPackField.tsx

The Cursor panel's Pack group: the collapsible style sections of installed packs, and the import affordance under them. Split out of `CursorPanel.tsx` by the panel pass so that file reads as the panel's flow (style, pack, size, motion, click) rather than as a list of async handlers.

Everything about the pack list lives here: the `listCursorPacks()` fetch, the `importCursorPack()` call, and the import error. `CursorPanel` passes only the selected pack id and a setter.

## PACK_SKELETON_COUNT

```ts
const PACK_SKELETON_COUNT = 3
```

How many skeleton tiles stand in while the list is in flight: one full row of `.e-tile-grid`, which is three at the 320px panel width. The skeleton is therefore exactly the shape of the first thing that replaces it, and nothing resizes when the real list lands (`.e-tile-skel` fixes the tile's height for the same reason). It was four while the packs were a sideways strip, and five before that.

## CursorPackField

```tsx
export function CursorPackField({ pack, onPick }: {
  pack: string; onPick: (id: string) => void;
}): JSX.Element
```

### Props

- `pack` - `CursorSettings.pack`, the selected pack id.
- `onPick` - writes it. Also called with the new pack's id right after a successful import, so importing selects what you just imported.

### Behavior

**Pack list.** `packs: CursorPackInfo[] | null` - `null` means the fetch is in flight; it resolves to the real list (built-in "Default" first, then imports) or, on rejection, `[]`. Three renders keyed on that:

- `null` -> `PACK_SKELETON_COUNT` `Shimmer` tiles carrying `.e-tile .e-tile-skel`, in a `.e-tile-grid`.
- `[]` (resolved empty, or rejected) -> one quiet `.e-hintline`: "No cursor packs yet. Import a pack folder." - pointing at the button below rather than offering a second retry affordance.
- non-empty -> `CursorPackGrid`, one collapsible section per pack category.

**Import.** The button (`.e-upload`, `IconFolderPlus`) opens the Tauri dialog plugin's folder picker (`{ directory: true, multiple: false }`), then calls `importCursorPack(dir)`. On success the returned `CursorPackInfo` is appended to local `packs` (so the grid updates with no re-fetch; `prev ?? []` covers an import finishing while the list was still loading) and selected. On failure the rejection message - or a generic fallback when it is not a string - shows as an `.e-errline` under the button, inside `AnimatePresence` (0.14s opacity/y-4, dropped under `useReducedMotion`). While in flight the button is disabled and its label reads "Importing...".

No client-side validation of the chosen folder happens here: `importCursorPack` (Rust) checks recognised filenames, real PNG bytes and `hotspots.json` shape, and this component surfaces whatever error string comes back.

**The import affordance is no longer a dashed box** (`.e-upload`, not the old `.e-upload-dashed`): a dashed outline is a drop target's idiom and this one opens a file dialog. It is now the same quiet raised plane as every other control, full panel width, with an icon, a label and a tooltip.

### Used by

- `src/editor/panels/CursorPanel.tsx` - the Pack group, rendered only for the Enhanced style.
