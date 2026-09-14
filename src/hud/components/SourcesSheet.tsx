import { AnimatePresence, motion } from "motion/react";
import { Dropdown, type DropOption } from "./Dropdown";
import { FROST } from "./StateSwap";
import { Camera, Chevron, Mic, Monitor } from "./icons";
import { TargetSheet } from "../devices/TargetSheet";
import { parseTarget, type DisplayInfo } from "../devices/selectDevices";

/** The take pill's Sources sheet: the three inputs, switchable without stopping the take (the
 *  2026-09-14 design's HUD half). It hangs UNDER the pill inside the same `.hud` surface, which
 *  grows for it (`useHudWindowSize`'s `SOURCES_HEIGHT`) and shrinks back when it closes, so the
 *  pill itself never changes width or moves.
 *
 *  The rows are the idle card's own pickers, verbatim: `Dropdown` `row` for camera and mic, and a
 *  row that flips the sheet to `TargetSheet` for the display, so nothing here is a second way to
 *  read the same device list. Picking applies at once (`onCam`/`onMic`/`onTarget` run the live
 *  switch) and closes the sheet - there is no Apply, and nothing to confirm.
 *
 *  The sheet arrives through the bar<->pill swap's own frost (`StateSwap`'s `FROST`), and so does
 *  the flip to the display list inside it. Closing is a hard cut, for the reason `Dropdown`'s menu
 *  documents: the window is already gliding shut around it, and an exit animation under that is a
 *  clip, not a fade.
 *
 *  Owns no state: the open menu, the display flip and every selection are `Hud`'s, the same
 *  values the idle card reads. */
export function SourcesSheet(p: {
  targets: DisplayInfo[]; displayId: string; onTarget: (id: string) => void;
  cameras: DropOption[]; camId: string; onCam: (id: string) => void;
  mics: DropOption[]; micId: string; onMic: (id: string) => void;
  menu: string | null; onMenu: (id: string) => void;
  sheet: boolean; onSheet: (open: boolean) => void;
}) {
  const target = p.targets.map((d, i) => ({ d, meta: parseTarget(d, i) })).find((r) => r.d.id === p.displayId)?.meta;
  return (
    <motion.div className="src-sheet" initial={FROST.frosted} animate={FROST.shown} transition={FROST.arrive}>
      <AnimatePresence mode="wait" initial={false}>
        <motion.div key={p.sheet ? "targets" : "rows"} className="src-body" initial={FROST.frosted}
          animate={FROST.shown} exit={{ ...FROST.frosted, transition: FROST.leave }} transition={FROST.arrive}>
          {p.sheet ? (
            <TargetSheet targets={p.targets} value={p.displayId} onBack={() => p.onSheet(false)}
              onPick={(id) => { p.onTarget(id); p.onSheet(false); }} />
          ) : (<>
            <button type="button" className="dd-row" title="Change what is being recorded" onClick={() => p.onSheet(true)}>
              <span className="ico"><Monitor /></span>
              <span className="dd-main"><span className="dd-label">{target?.title ?? "-"}</span></span>
              <span className="chev right"><Chevron /></span>
            </button>
            <Dropdown row icon={<Camera />} value={p.camId} options={p.cameras} open={p.menu === "src-cam"}
              onToggle={() => p.onMenu("src-cam")} onPick={p.onCam} />
            <Dropdown row icon={<Mic />} value={p.micId} options={p.mics} open={p.menu === "src-mic"}
              onToggle={() => p.onMenu("src-mic")} onPick={p.onMic} />
          </>)}
        </motion.div>
      </AnimatePresence>
    </motion.div>
  );
}
