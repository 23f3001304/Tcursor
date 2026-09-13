import { useEffect, useRef, useState } from "react";
import type { CursorPackInfo } from "../../lib/ipc";
import { fileSrc } from "../../lib/ipc";
import { busyPose } from "../stage/cursorBusy";
import { GlyphPlate, PackTile } from "./PackTile";

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
    <PackTile selected={selected} onPick={onPick} label={pack.name}
      title={pack.builtin ? `${pack.name} (built in)` : pack.name}
      onHoverStart={() => setHovered(true)} onHoverEnd={() => setHovered(false)}>
      <GlyphPlate src={spriteSrc(pack, KINDS[0], 0)} imgRef={img} />
    </PackTile>
  );
}

/** The pack picker: built-in packs first under a dim group label, then the imported ones. Each
 *  tile rests on the pack's arrow and, while hovered, walks its nine states so the user sees what
 *  they are choosing before choosing it.
 *
 *  One STRIP per group since the usability pass: fifteen built-in packs four-up was four rows of
 *  the panel, which is most of the Cursor panel's budget spent before Size and Motion get a look
 *  in. Sideways, the same fifteen cost one. Every tile stays a plain button in the tab order, so
 *  Tab still reaches each pack in visual order and the browser scrolls the focused one into view. */
export function CursorPackGrid({ packs, selected, onPick }: {
  packs: CursorPackInfo[]; selected: string; onPick: (id: string) => void;
}) {
  const groups: [string, CursorPackInfo[]][] = [
    ["Built in", packs.filter((p) => p.builtin)],
    ["Imported", packs.filter((p) => !p.builtin)],
  ];
  return (
    <>
      {groups.filter(([, list]) => list.length > 0).map(([label, list]) => (
        <div key={label} className="e-tile-section">
          <span className="e-tile-group">{label}</span>
          <div className="e-tile-strip">
            {list.map((p) => (
              <CursorPack key={p.id} pack={p} selected={selected === p.id} onPick={() => onPick(p.id)} />
            ))}
          </div>
        </div>
      ))}
    </>
  );
}
