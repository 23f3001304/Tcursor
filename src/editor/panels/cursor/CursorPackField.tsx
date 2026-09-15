import { useCallback, useEffect, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { IconFolderPlus } from "@tabler/icons-react";
import type { CursorPackInfo } from "../../../shared/ipc";
import { listCursorPacks, importCursorPack, createPackTemplate } from "../../../shared/ipc";
import { Shimmer } from "../../timeline/lanes/Shimmer";
import { CursorPackGrid } from "./CursorPackGrid";

const PACK_SKELETON_COUNT = 3;
const HINT_MOTION = {
  initial: { opacity: 0, y: -4 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -4 },
  transition: { duration: 0.14 },
};

export function CursorPackField({ pack, onPick }: { pack: string; onPick: (id: string) => void }) {
  const [packs, setPacks] = useState<CursorPackInfo[] | null>(null);
  const [importing, setImporting] = useState(false);
  const [err, setErr] = useState("");
  const [templating, setTemplating] = useState(false);
  const [templateResult, setTemplateResult] = useState<{ ok: boolean; text: string } | null>(null);
  const still = useReducedMotion();

  const loadPacks = useCallback(() => {
    setPacks(null);
    listCursorPacks()
      .then(setPacks)
      .catch(() => setPacks([]));
  }, []);
  useEffect(loadPacks, [loadPacks]);

  const handleImport = async () => {
    setErr("");
    const dir = await open({ directory: true, multiple: false, title: "Choose a cursor pack folder" }).catch(
      () => null,
    );
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

  const handleCreateTemplate = async () => {
    setTemplateResult(null);
    const dir = await open({
      directory: true,
      multiple: false,
      title: "Choose a folder for the pack template",
    }).catch(() => null);
    if (!dir || Array.isArray(dir)) return;
    setTemplating(true);
    try {
      const created = await createPackTemplate(dir);
      await revealItemInDir(created).catch(() => {});
      setTemplateResult({ ok: true, text: `Created ${created}` });
    } catch (e) {
      setTemplateResult({
        ok: false,
        text: typeof e === "string" ? e : "Could not create the pack template.",
      });
    } finally {
      setTemplating(false);
    }
  };

  return (
    <>
      <div className="e-grp">
        <span className="e-sechead">Cursor style pack</span>
        {packs === null ? (
          <div className="e-tile-grid">
            {Array.from({ length: PACK_SKELETON_COUNT }, (_, i) => (
              <Shimmer key={i} className="e-tile e-tile-skel" />
            ))}
          </div>
        ) : packs.length === 0 ? (
          <p className="e-hintline">No cursor packs yet. Import a pack folder.</p>
        ) : (
          <CursorPackGrid packs={packs} selected={pack} onPick={onPick} />
        )}
        <button
          type="button"
          className="e-upload"
          onClick={handleImport}
          disabled={importing}
          title="Choose a folder of cursor PNGs to add as a pack"
        >
          <IconFolderPlus size={14} /> {importing ? "Importing..." : "Import pack..."}
        </button>
        <AnimatePresence>
          {err && (
            <motion.p className="e-errline" {...(still ? {} : HINT_MOTION)}>
              {err}
            </motion.p>
          )}
        </AnimatePresence>
      </div>

      <div className="e-grp">
        <span className="e-sechead">How packs work</span>
        <p className="e-hintline">
          A pack is a folder with one PNG per cursor state, plus a pack.json (name and an optional busy
          animation) and a hotspots.json for click points. Any state you skip falls back to the built-in
          sprite. "Import pack..." above adds a folder like this one to TCursor.
        </p>
        <button
          type="button"
          className="e-ghostbtn"
          onClick={handleCreateTemplate}
          disabled={templating}
          title="Write a starter pack folder with placeholder sprites, ready to edit and import"
        >
          {templating ? "Creating..." : "Create pack template"}
        </button>
        <AnimatePresence>
          {templateResult && (
            <motion.p
              className={templateResult.ok ? "e-hintline" : "e-errline"}
              {...(still ? {} : HINT_MOTION)}
            >
              {templateResult.text}
            </motion.p>
          )}
        </AnimatePresence>
      </div>
    </>
  );
}
