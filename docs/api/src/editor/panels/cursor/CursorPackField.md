# src/editor/panels/cursor/CursorPackField.tsx

The Cursor panel's Pack group: the collapsible style sections of installed packs, the import affordance under them, and (M8(c)) a second group underneath explaining the pack folder format with a "Create pack template" button. Split out of `CursorPanel.tsx` by the panel pass so that file reads as the panel's flow (style, pack, size, motion, click) rather than as a list of async handlers.

Everything about the pack list lives here: the `listCursorPacks()` fetch, the `importCursorPack()` call, the import error, and the `createPackTemplate()` call with its own confirmation/error line. `CursorPanel` passes only the selected pack id and a setter.

Returns a fragment of two `.e-grp` blocks rather than one - "Cursor style pack" (the grid + import affordance, unchanged) and "How packs work" (new) - so the template explainer reads as its own group with its own heading rather than a tail appended to the pack picker's.

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

**How packs work (M8(c)).** A second `.e-grp` below the pack group: an `.e-hintline` paragraph stating the folder contract in three sentences (one PNG per state, `pack.json` plus an optional `hotspots.json`, missing states fall back to the built-in sprite), then an `.e-ghostbtn` "Create pack template". Clicking it:

1. Opens a *second*, independent folder picker (`{ directory: true, multiple: false, title: "Choose a folder for the pack template" }`) - deliberately not the same call as the import picker, since this one is a destination to write into, not a pack to read.
2. Calls `createPackTemplate(dir)` (`src/shared/ipc.ts`), which returns the created folder's absolute path.
3. Reveals it with `revealItemInDir` (`@tauri-apps/plugin-opener`), swallowing any reveal failure (`.catch(() => {})`) - the template still exists and the confirmation line still shows even if the OS can't open Explorer.
4. Shows a one-line result: `templateResult: { ok: boolean; text: string } | null`, rendered as `.e-hintline` (`"Created <path>"`) when `ok`, or `.e-errline` (the backend's own error string, or a generic fallback when the rejection isn't a string) when not - the same `AnimatePresence`/`HINT_MOTION` fade as the import error.

While the write is in flight (`templating`) the button is disabled and reads "Creating...". Dismissing the folder picker without choosing anything (`null`) is a silent no-op, same as `handleImport`.

### Used by

- `src/editor/panels/cursor/CursorPanel.tsx` - the Pack group, rendered only for the Enhanced style.
