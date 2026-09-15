import type { CursorPackInfo } from "../../../shared/ipc";

export const CATEGORY_ORDER = ["Classic", "Glass and glow", "Playful", "Drawn", "Retro", "Imported"];

export interface PackCategory {
  name: string;
  packs: CursorPackInfo[];
}

function rank(name: string): number {
  const i = CATEGORY_ORDER.indexOf(name);
  return i >= 0 ? i : CATEGORY_ORDER.length;
}

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
