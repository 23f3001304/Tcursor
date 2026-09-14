import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { open } from "@tauri-apps/plugin-dialog";
import { IconFolderPlus } from "@tabler/icons-react";
import type { CursorPackInfo } from "../../lib/ipc";
import { listCursorPacks, importCursorPack } from "../../lib/ipc";
import { Shimmer } from "../timeline/Shimmer";
import { CursorPackGrid } from "./CursorPackGrid";

// Loading-skeleton tile count: one full row of the wrapping grid (three at the 320px panel width),
// so the skeleton is exactly the shape of the first thing that replaces it.
const PACK_SKELETON_COUNT = 3;
const HINT_MOTION = { initial: { opacity: 0, y: -4 }, animate: { opacity: 1, y: 0 }, exit: { opacity: 0, y: -4 }, transition: { duration: 0.14 } };

/** The Cursor panel's Pack group: the grid of installed packs and the import affordance under it,
 *  with the list fetch, the import call and the import error all owned here so `CursorPanel` stays
 *  a flat read of the panel's flow. Split out of `CursorPanel.tsx` in the panel pass. */
export function CursorPackField({ pack, onPick }: { pack: string; onPick: (id: string) => void }) {
  // `null` = still loading; `[]` after resolving is a genuine (if unlikely - there's always a
  // built-in "Default" pack) empty/error state, both shown as the same quiet empty message.
  const [packs, setPacks] = useState<CursorPackInfo[] | null>(null);
  const [importing, setImporting] = useState(false);
  const [err, setErr] = useState("");
  const still = useReducedMotion();

  const loadPacks = useCallback(() => {
    setPacks(null);
    listCursorPacks().then(setPacks).catch(() => setPacks([]));
  }, []);
  useEffect(loadPacks, [loadPacks]);

  const handleImport = async () => {
    setErr("");
    const dir = await open({ directory: true, multiple: false, title: "Choose a cursor pack folder" }).catch(() => null);
    if (!dir || Array.isArray(dir)) return;
    setImporting(true);
    try {
      const info = await importCursorPack(dir);
      setPacks((prev) => [...(prev ?? []), info]);
      onPick(info.id);
    } catch (e) {
      setErr(typeof e === "string" ? e : "Import failed. Check the folder has cursor PNGs.");
    } finally {
      setImporting(false);
    }
  };

  return (
    <div className="e-grp">
      <span className="e-sechead">Cursor style pack</span>
      {packs === null ? (
        <div className="e-tile-grid">
          {Array.from({ length: PACK_SKELETON_COUNT }, (_, i) => <Shimmer key={i} className="e-tile e-tile-skel" />)}
        </div>
      ) : packs.length === 0 ? (
        <p className="e-hintline">No cursor packs yet. Import a pack folder.</p>
      ) : (
        <CursorPackGrid packs={packs} selected={pack} onPick={onPick} />
      )}
      <button type="button" className="e-upload" onClick={handleImport} disabled={importing}
        title="Choose a folder of cursor PNGs to add as a pack">
        <IconFolderPlus size={14} /> {importing ? "Importing..." : "Import pack..."}
      </button>
      <AnimatePresence>
        {err && <motion.p className="e-errline" {...(still ? {} : HINT_MOTION)}>{err}</motion.p>}
      </AnimatePresence>
    </div>
  );
}
