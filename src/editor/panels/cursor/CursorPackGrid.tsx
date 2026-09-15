import { useEffect, useRef, useState } from "react";
import type { CursorPackInfo } from "../../../shared/ipc";
import { fileSrc } from "../../../shared/ipc";
import { CategorySection, defaultOpenIndex } from "../../controls/Controls";
import { busyPose } from "../../stage/cursor/cursorBusy";
import { GlyphPlate, PackTile } from "../PackTile";
import { packCategories } from "./packCategories";

const KINDS = [
  "arrow",
  "ibeam",
  "hand",
  "resize_ns",
  "resize_ew",
  "resize_nwse",
  "resize_nesw",
  "move",
  "busy",
];
const KIND_MS = 400;
const LAP_MS = KINDS.length * KIND_MS;
const BUSY_INDEX = KINDS.indexOf("busy");

function spriteSrc(pack: CursorPackInfo, kind: string, frame: number): string {
  if (!pack.dir) return "";
  const file =
    kind === "busy" && (pack.busy?.frames ?? 0) > 0
      ? `busy_${String(frame).padStart(2, "0")}.png`
      : pack.files[kind];
  return file ? fileSrc(`${pack.dir}\\${file}`) : "";
}

function useHoverCycle(
  pack: CursorPackInfo,
  hovered: boolean,
  img: React.RefObject<HTMLImageElement | null>,
) {
  useEffect(() => {
    const rest = () => {
      const el = img.current;
      if (!el) return;
      el.src = spriteSrc(pack, KINDS[0], 0);
      el.style.transform = "none";
    };
    if (!hovered) {
      rest();
      return;
    }
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
        if (src && src !== shown) {
          el.src = src;
          shown = src;
        }
        el.style.transform = pose ? `rotate(${pose.angleDeg}deg) scale(${pose.scale})` : "none";
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(raf);
      rest();
    };
  }, [hovered, pack, img]);
}

function CursorPack({
  pack,
  selected,
  onPick,
}: {
  pack: CursorPackInfo;
  selected: boolean;
  onPick: () => void;
}) {
  const [hovered, setHovered] = useState(false);
  const img = useRef<HTMLImageElement | null>(null);
  useHoverCycle(pack, hovered, img);
  return (
    <PackTile
      selected={selected}
      onPick={onPick}
      label={pack.name}
      title={pack.name}
      onHoverStart={() => setHovered(true)}
      onHoverEnd={() => setHovered(false)}
    >
      <GlyphPlate src={spriteSrc(pack, KINDS[0], 0)} imgRef={img} />
    </PackTile>
  );
}

export function CursorPackGrid({
  packs,
  selected,
  onPick,
}: {
  packs: CursorPackInfo[];
  selected: string;
  onPick: (id: string) => void;
}) {
  const cats = packCategories(packs);
  const open = defaultOpenIndex(cats.map((c) => c.packs.some((p) => p.id === selected)));
  return (
    <div className="e-secstack">
      {cats.map((c, i) => (
        <CategorySection
          key={c.name}
          id={`cursorpack.${c.name}`}
          label={c.name}
          count={c.packs.length}
          selectedName={c.packs.find((p) => p.id === selected)?.name ?? null}
          defaultOpen={i === open}
        >
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
