// How the Cursor panel's packs are grouped into its collapsible sections. The grouping KEY is
// `CursorPackInfo.category`, which every pack states in its own `pack.json` (the embedded default
// pack states it in Rust, and a pack with no category at all lists as "Imported") - so a new pack
// ships by dropping a folder in, and nothing in the frontend holds a list of pack ids.
//
// "Built in vs imported" was the old split, and the owner's read of it was that it says nothing
// about what a pack LOOKS like, which is the only thing a user is choosing between here.
import type { CursorPackInfo } from "../../lib/ipc";

/** The curated sections, in the order the panel shows them: the plain arrows first (what most
 *  recordings want), then the loud ones, then the drawn and dated ones, and the user's own last.
 *  A category outside this list is not an error - it sorts alphabetically after these. */
export const CATEGORY_ORDER = ["Classic", "Glass and glow", "Playful", "Drawn", "Retro", "Imported"];

/** One section of the pack picker. */
export interface PackCategory { name: string; packs: CursorPackInfo[] }

/** Where `name` sorts. Known categories keep `CATEGORY_ORDER`; anything else goes after all of
 *  them, alphabetically, so an unknown category is visible rather than silently dropped. */
function rank(name: string): number {
  const i = CATEGORY_ORDER.indexOf(name);
  return i >= 0 ? i : CATEGORY_ORDER.length;
}

/** `packs` split into the picker's sections. Packs keep the backend's order inside a section (the
 *  embedded default first, then bundled, then imported), empty sections never appear, and a pack
 *  whose category is missing or blank joins "Imported" - the same fallback Rust applies, repeated
 *  here only so a hand-edited reply cannot produce a nameless section. */
export function packCategories(packs: CursorPackInfo[]): PackCategory[] {
  const by = new Map<string, CursorPackInfo[]>();
  for (const p of packs) {
    const name = p.category?.trim() || "Imported";
    const list = by.get(name);
    if (list) list.push(p);
    else by.set(name, [p]);
  }
  return [...by.entries()]
    .map(([name, list]) => ({ name, packs: list }))
    .sort((a, b) => rank(a.name) - rank(b.name) || a.name.localeCompare(b.name));
}
