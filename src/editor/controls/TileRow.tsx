import { useEffect, useRef, type ReactNode } from "react";
import { motion, useReducedMotion } from "motion/react";

// The app's press spring, the same one `PackTile` uses - a strip tile is that tile at a smaller
// size, so it presses the same way.
const PRESS = { type: "spring" as const, stiffness: 500, damping: 30 };

// One horizontally scrolling row of picture tiles, the panel pass's answer to a preset GRID: a
// 52-tile wallpaper library laid out three-up costs roughly 1000px of a 620px panel, while the
// same library as five 55px rows costs 295px and still shows every tile - the scrolling moves
// from the panel (which has nowhere to go) into the row (which does).
//
// Keyboard follows the WAI-ARIA listbox pattern with selection-following-focus: the row is one
// tab stop, arrows walk it, and walking it applies the tile - so a user can audition a whole
// group without ever leaving the keyboard.

/** One tile: `content` is whatever paints the 64x36 face (a thumbnail, a swatch, a gradient). */
export interface RowTile { id: string; name: string; content: ReactNode }

/** The tile index an arrow/Home/End press should move to, or `null` for any other key.
 *  CLAMPS at both ends rather than wrapping (unlike `segmentedNextIndex`): a row can hold a dozen
 *  tiles and running off the end of a scrolling strip should stop, not teleport to the far end. */
export function rowNextIndex(key: string, index: number, length: number): number | null {
  if (length === 0) return null;
  const at = index < 0 ? 0 : index;
  switch (key) {
    case "ArrowRight": case "ArrowDown": return Math.min(at + 1, length - 1);
    case "ArrowLeft": case "ArrowUp": return Math.max(at - 1, 0);
    case "Home": return 0;
    case "End": return length - 1;
    default: return null;
  }
}

export function TileRow({ label, ariaLabel, tiles, selectedId, onSelect }: {
  /** The row's own 11px label, which is what a group heading became (the group name). */
  label?: string;
  ariaLabel: string;
  tiles: RowTile[];
  /** `null` = nothing in THIS row is active, which is normal for a row that is not the live kind. */
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  const wrap = useRef<HTMLDivElement>(null);
  const still = useReducedMotion();
  const index = tiles.findIndex((t) => t.id === selectedId);

  // Scroll the chosen tile into view - on mount (so reopening a panel lands on what is actually
  // selected, however deep in the strip it sits) and after an arrow key moves the selection.
  // `nearest` on both axes: the row must never scroll the PANEL, only itself. `?.` because jsdom
  // does not implement scrollIntoView.
  useEffect(() => {
    wrap.current?.querySelector<HTMLElement>('[aria-selected="true"]')
      ?.scrollIntoView?.({ block: "nearest", inline: "nearest" });
  }, [selectedId]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    const next = rowNextIndex(e.key, index, tiles.length);
    if (next === null) return;
    e.preventDefault();
    onSelect(tiles[next].id);
    wrap.current?.querySelectorAll<HTMLButtonElement>('[role="option"]')[next]?.focus();
  };

  return (
    <div className="e-tilerow-wrap">
      {label && <span className="e-tilerow-label">{label}</span>}
      <div ref={wrap} className="e-tilerow" role="listbox" aria-label={ariaLabel} onKeyDown={onKeyDown}>
        {tiles.map((t, i) => {
          const on = t.id === selectedId;
          return (
            // Roving tabindex: the selected tile is the row's one tab stop, and the first tile
            // stands in when this row holds no selection at all.
            <motion.button key={t.id || "(default)"} type="button" role="option" aria-selected={on}
              title={t.name} tabIndex={on || (index < 0 && i === 0) ? 0 : -1}
              className={`e-rowtile${on ? " on" : ""}`} onClick={() => onSelect(t.id)}
              whileTap={still ? undefined : { scale: 0.96 }} transition={PRESS}>
              {t.content}
              {/* The name rides in over the face on hover/focus, so a row at rest is all picture.
                  Never hover-ONLY information: `title` above carries it too. */}
              <span className="e-rowtile-cap">{t.name}</span>
            </motion.button>
          );
        })}
      </div>
    </div>
  );
}
