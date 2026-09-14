# src/editor/panels/packCategories.ts

How the Cursor panel's packs are grouped into its collapsible sections. Added by the **arrangements pass** (2026-09-14), split out of `CursorPackGrid.tsx` so the ordering rule has a test of its own and the grid stays a render.

**The grouping key is data, never a list of ids.** Every pack states its own `category` in `pack.json`; the embedded default set states it in Rust (`packlist::DEFAULT_PACK_CATEGORY`), because it is the one pack with no manifest. So a new pack joins a section by shipping a folder, and nothing in the frontend has to learn its name. `src-tauri/assets/cursorpacks/README.md` is where that field is documented for whoever adds the next pack.

**Why style and not provenance.** The picker used to split "Built in" from "Imported". The owner's read of that: it says nothing about what a pack LOOKS like, which is the only thing anyone is choosing between here. Fifteen bundled packs under one heading is a wall; the same fifteen under Classic / Glass and glow / Playful / Drawn / Retro is a decision. "Imported" survives as one section among those, because *where a pack came from* is still the one thing a user's own pack has in common.

## CATEGORY_ORDER

```ts
export const CATEGORY_ORDER: string[]
```

The curated sections, in the order the panel shows them: `Classic`, `Glass and glow`, `Playful`, `Drawn`, `Retro`, `Imported`. Plain arrows first (what most recordings want), then the loud ones, then the drawn and dated ones, and the user's own last.

Mirrored in Rust by `packlist_tests::KNOWN_CATEGORIES`, which fails the build if a bundled `pack.json` ever declares something outside this list.

## PackCategory

```ts
export interface PackCategory { name: string; packs: CursorPackInfo[] }
```

One section of the pack picker: its heading and the packs under it.

## rank

```ts
function rank(name: string): number
```

Where a category sorts. Known ones keep their `CATEGORY_ORDER` index; anything else gets `CATEGORY_ORDER.length`, so an unknown category lands after all of them rather than being dropped. A pack whose category nobody recognises is still a pack the user can pick.

## packCategories

```ts
export function packCategories(packs: CursorPackInfo[]): PackCategory[]
```

`packs` split into the picker's sections.

- Packs keep the backend's order INSIDE a section (the embedded default first, then bundled, then imported), because that order is already stable and alphabetical per source.
- An empty section never appears - a user with no imports sees no "Imported" heading.
- A pack whose category is missing or blank joins `Imported`. Rust applies the same fallback (`packlist::category_or_imported`); repeating it here only keeps a hand-edited or older reply from producing a section with no name at all.
- Unknown categories sort alphabetically after the curated ones, which is what the `localeCompare` tiebreak is for (`Array.prototype.sort` is stable, so the curated ones keep `CATEGORY_ORDER`).

### Behaviors

- `groups by the pack's own category, in the curated order, never by builtin`.
- `keeps the backend's order inside a section`.
- `omits a category nothing is in, rather than showing an empty heading`.
- `puts an unknown category after the curated ones, alphabetically`.
- `lists a pack with a missing or blank category under Imported`.

### Used by

- `src/editor/panels/CursorPackGrid.tsx` - one `CategorySection` per returned entry.
