import { useEffect, useRef, useState } from "react";
import type { CursorPackInfo } from "../../lib/ipc";
import { fileSrc } from "../../lib/ipc";
import { CategorySection, defaultOpenIndex } from "../controls/Controls";
import { busyPose } from "../stage/cursorBusy";
import { GlyphPlate, PackTile } from "./PackTile";
import { packCategories } from "./packCategories";

// The nine cursor states, in the order a hovered tile walks through them. Same wire names the
// backend uses for the PNG filenames, so a tile's sprite path is just `<dir>/<kind>.png`.
const KINDS = ["arrow", "ibeam", "hand", "resize_ns", "resize_ew", "resize_nwse", "resize_nesw", "move", "busy"];
const KIND_MS = 400; // how long the hover preview holds each state
const LAP_MS = KINDS.length * KIND_MS;
const BUSY_INDEX = KINDS.indexOf("busy");

/** One pack's sprite file, through the asset protocol. The backend already resolved every name
 *  into `pack.files` - legacy aliases (the embedded pack spells its arrow `pointer.png`) and the
 *  busy-is-arrow substitution included - so this only has to special-case explicit busy frames,
 *  which are numbered rather than named. Empty when the pack has no folder, or does not ship
 *  that kind. */
function spriteSrc(pack: CursorPackInfo, kind: string, frame: number): string {
  if (!pack.dir) return "";
  const file = kind === "busy" && (pack.busy?.frames ?? 0) > 0
    ? `busy_${String(frame).padStart(2, "0")}.png`
    : pack.files[kind];
  return file ? fileSrc(`${pack.dir}\\${file}`) : "";
}

/** Walk `img` through the nine states while `hovered`, animating the busy one with the SAME
 *  `busyPose` the export and the canvas preview run, so a tile shows exactly what picking that
 *  pack will render. Writes `src`/`transform` straight onto the element: `src` only when the file
 *  actually changes (or the browser re-decodes every frame), `transform` every frame. The element's
 *  size and offset are `GlyphPlate`'s fitted ones and are never touched here, so every state of
 *  every pack stays the same visual size. */
function useHoverCycle(pack: CursorPackInfo, hovered: boolean, img: React.RefObject<HTMLImageElement | null>) {
  useEffect(() => {
    const rest = () => {
      const el = img.current;
      if (!el) return;
      el.src = spriteSrc(pack, KINDS[0], 0);
      el.style.transform = "none";
    };
    if (!hovered) { rest(); return; }
    let raf = 0;
    let shown = "";
    const start = performance.now();
    const tick = (now: number) => {
      const el = img.current;
      if (el) {
        const lap = (now - start) % LAP_MS;
        const i = Math.floor(lap / KIND_MS);
        const pose = i === BUSY_INDEX && pack.busy ? busyPose(pack.busy, lap - i * KIND_MS) : null;
        const src = spriteSrc(pack, KINDS[i], pose?.frame ?? 0);
        // A kind this pack does not ship keeps the previous state up rather than blanking the
        // tile: the renderer falls back to the embedded sprite there, which we cannot display.
        if (src && src !== shown) { el.src = src; shown = src; }
        el.style.transform = pose ? `rotate(${pose.angleDeg}deg) scale(${pose.scale})` : "none";
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => { cancelAnimationFrame(raf); rest(); };
  }, [hovered, pack, img]);
}

function CursorPack({ pack, selected, onPick }: {
  pack: CursorPackInfo; selected: boolean; onPick: () => void;
}) {
  const [hovered, setHovered] = useState(false);
  const img = useRef<HTMLImageElement | null>(null);
  useHoverCycle(pack, hovered, img);
  return (
    // No "(built in)" suffix any more: the SECTION says where a pack comes from and what it looks
    // like, so the tooltip would only repeat the heading above it.
    <PackTile selected={selected} onPick={onPick} label={pack.name} title={pack.name}
      onHoverStart={() => setHovered(true)} onHoverEnd={() => setHovered(false)}>
      <GlyphPlate src={spriteSrc(pack, KINDS[0], 0)} imgRef={img} />
    </PackTile>
  );
}

/** The pack picker: one collapsible section per STYLE (Classic, Glass and glow, Playful, Drawn,
 *  Retro, Imported), each holding a wrapping grid of tiles three to a row. Each tile rests on the
 *  pack's arrow and, while hovered, walks its nine states so the user sees what it draws before
 *  choosing it.
 *
 *  Sections, not the sideways strips the usability pass shipped: the owner's read of those was
 *  that a picture library you have to flick through hides most of itself, and that "built in vs
 *  imported" was not a difference anyone is choosing between. A category is. Only the section
 *  holding the current pack opens by itself, so the panel is shorter than the strips were AND
 *  every tile in it is fully visible with its name under it. */
export function CursorPackGrid({ packs, selected, onPick }: {
  packs: CursorPackInfo[]; selected: string; onPick: (id: string) => void;
}) {
  const cats = packCategories(packs);
  const open = defaultOpenIndex(cats.map((c) => c.packs.some((p) => p.id === selected)));
  return (
    <div className="e-secstack">
      {cats.map((c, i) => (
        <CategorySection key={c.name} id={`cursorpack.${c.name}`} label={c.name} count={c.packs.length}
          selectedName={c.packs.find((p) => p.id === selected)?.name ?? null} defaultOpen={i === open}>
          <div className="e-tile-grid">
            {c.packs.map((p) => (
              <CursorPack key={p.id} pack={p} selected={selected === p.id} onPick={() => onPick(p.id)} />
            ))}
          </div>
        </CategorySection>
      ))}
    </div>
  );
}
