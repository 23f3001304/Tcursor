import type { ReactNode, RefCallback } from "react";
import { AnimatePresence, motion } from "motion/react";
import { CamTile } from "./CamTile";
import { Dropdown, type DropOption } from "./Dropdown";
import { RecordButton } from "./RecordButton";
import { TargetSheet } from "../devices/TargetSheet";
import { parseTarget, type DisplayInfo } from "../devices/selectDevices";
import { TcursorMark } from "../../lib/TcursorMark";
import { Camera, CameraOff, Chevron, CloseIcon, FolderOpen, Gamepad, Gear, Mic, MicOff, MinIcon, Monitor, Palette, Speaker, SpeakerOff } from "./icons";

// design/premium-pass D6: press spring for the toggle row (Record has its own, RecordButton.tsx).
const TOGGLE_PRESS = { whileTap: { scale: 0.96 }, transition: { type: "spring" as const, stiffness: 500, damping: 30 } };
// The card <-> sheet flip: the same frost the bar<->pill swap uses (StateSwap.tsx), shallower.
// Settings and Preferences ride it too (owner 2026-09-14: "do it like how we are handling display
// changes"), so the three body states share one interstate rather than each having its own.
const FROSTED = { opacity: 0, filter: "blur(10px)", scale: 0.97 };
const SHOWN = { opacity: 1, filter: "blur(0px)", scale: 1 };
const FLIP = { scale: { type: "spring" as const, stiffness: 380, damping: 18 }, filter: { duration: 0.22 }, opacity: { duration: 0.16 } };

export interface Toggles { camOn: boolean; micOn: boolean; sysOn: boolean; gameMode: boolean }

/** The idle HUD as a vertical card (owner, 2026-09-14, over a one-row bar): the header with the
 *  brand and the window buttons, a wide webcam preview, the three sources as rows (camera and
 *  mic as in-place dropdowns, the screen as a row that flips the card to `TargetSheet`), the four
 *  source toggles as one labelled segmented row, and Record as the single accent button across
 *  the bottom. 360 wide.
 *
 *  The body has three states, all on the same flip: the sources, the display picker, and a panel
 *  (`panelBody`, Settings or Preferences) - so opening Settings is the card frosting to another
 *  body, not a window becoming a box.
 *
 *  Owns no state: selection, menus, the sheet, the panel and the toggles are all `Hud`'s, so the
 *  take flow reads the same values this card shows. */
export function IdleCard(p: {
  banner: string | null; exporting: boolean; pct: number;
  onOpenProject: () => void; onPreferences: () => void; onSettings: () => void; onMinimize: () => void; onClose: () => void;
  camRef: RefCallback<HTMLVideoElement>; camLive: boolean;
  cameras: DropOption[]; camId: string; onCam: (id: string) => void;
  targets: DisplayInfo[]; displayId: string; onTarget: (id: string) => void;
  mics: DropOption[]; micId: string; onMic: (id: string) => void;
  menu: string | null; onMenu: (id: string) => void;
  sheet: boolean; onSheet: (open: boolean) => void;
  panel: string | null; panelBody: ReactNode;
  toggles: Toggles; onToggle: (key: keyof Toggles) => void;
  onRecord: () => void;
}) {
  const t = p.toggles;
  const target = p.targets.map((d, i) => ({ d, meta: parseTarget(d, i) })).find((r) => r.d.id === p.displayId)?.meta;
  const sub = target ? [target.resolution?.replace("x", " by "), target.primary ? "Primary" : null].filter(Boolean).join(" · ") : "";
  return (
    <div className="card">
      <div className="card-head" data-tauri-drag-region>
        <span className="brand"><span className="brand-mark"><TcursorMark size={11} dotColor="var(--accent, #ef4444)" state="idle" /></span>TCursor</span>
        {/* A recording failure, an OS-ended take, or an export failure - one slot covers all. */}
        {p.banner && <span className="banner" title={p.banner}>{"⚠"} {p.banner}</span>}
        <span className="winctrls">
          {!p.exporting && (<>
            <button className="winbtn" title="Open Project" onClick={p.onOpenProject}><FolderOpen /></button>
            <button className="winbtn" title="Preferences" onClick={p.onPreferences}><Palette /></button>
            <button className="winbtn gear" title="Settings" onClick={p.onSettings}><Gear /></button>
            <span className="winsep" />
          </>)}
          <button className="winbtn" title="Minimize" onClick={p.onMinimize}><MinIcon /></button>
          <button className="winbtn close" title="Close" onClick={p.onClose}><CloseIcon /></button>
        </span>
      </div>
      <AnimatePresence mode="wait" initial={false}>
        <motion.div key={p.panel ?? (p.sheet ? "sheet" : "card")} className="card-body" initial={FROSTED} animate={SHOWN}
          exit={{ ...FROSTED, transition: { duration: 0.12 } }} transition={FLIP}>
          {p.panel ? p.panelBody : p.sheet ? (
            <TargetSheet targets={p.targets} value={p.displayId} onBack={() => p.onSheet(false)}
              onPick={(id) => { p.onTarget(id); p.onSheet(false); }} />
          ) : (<>
            <CamTile camRef={p.camRef} camOn={t.camOn} camLive={p.camLive} shape="wide" />
            <Dropdown row icon={<Camera />} value={p.camId} options={p.cameras} open={p.menu === "cam"}
              onToggle={() => p.onMenu("cam")} onPick={p.onCam} />
            <button type="button" className="dd-row" title="Choose what to record" onClick={() => p.onSheet(true)}>
              <span className="ico"><Monitor /></span>
              <span className="dd-main">
                <span className="dd-label">{target?.title ?? "-"}</span>
                {sub && <span className="dd-sub">{sub}</span>}
              </span>
              <span className="chev right"><Chevron /></span>
            </button>
            <Dropdown row icon={<Mic />} value={p.micId} options={p.mics} open={p.menu === "mic"}
              onToggle={() => p.onMenu("mic")} onPick={p.onMic} />
            <div className="tog-row" role="group" aria-label="Sources">
              <motion.button className={`tog ${t.camOn ? "on" : ""}`} title={t.camOn ? "Camera on" : "Camera off"} onClick={() => p.onToggle("camOn")} {...TOGGLE_PRESS}>{t.camOn ? <Camera /> : <CameraOff />}Camera</motion.button>
              <motion.button className={`tog ${t.micOn ? "on" : ""}`} title={t.micOn ? "Microphone on" : "Microphone off"} onClick={() => p.onToggle("micOn")} {...TOGGLE_PRESS}>{t.micOn ? <Mic /> : <MicOff />}Mic</motion.button>
              <motion.button className={`tog ${t.sysOn ? "on" : ""}`} title={t.sysOn ? "System audio on" : "System audio off"} onClick={() => p.onToggle("sysOn")} {...TOGGLE_PRESS}>{t.sysOn ? <Speaker /> : <SpeakerOff />}System</motion.button>
              <motion.button className={`tog ${t.gameMode ? "on" : ""}`} title={t.gameMode ? "Compatibility encoder (on): legacy CPU recording" : "Compatibility encoder: switch on only if GPU recording has issues"} onClick={() => p.onToggle("gameMode")} {...TOGGLE_PRESS}><Gamepad />Compat</motion.button>
            </div>
            <RecordButton disabled={p.exporting} onClick={p.onRecord} label={p.exporting ? `Exporting… ${p.pct}%` : "Record"} />
          </>)}
        </motion.div>
      </AnimatePresence>
    </div>
  );
}
