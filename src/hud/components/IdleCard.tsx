import type { ReactNode, RefCallback } from "react";
import { AnimatePresence, motion } from "motion/react";
import { CamTile } from "./CamTile";
import { Dropdown, type DropOption } from "./Dropdown";
import { RecordButton } from "./RecordButton";
import { TargetSheet } from "../devices/TargetSheet";
import { parseTarget, type DisplayInfo } from "../devices/selectDevices";
import { TcursorMark } from "../../shared/brand/TcursorMark";
import { SourceToggles, type Toggles } from "./SourceToggles";
import { Camera, Chevron, CloseIcon, FolderOpen, Gear, Mic, MinIcon, Monitor, Palette } from "./icons";

const FROSTED = { opacity: 0, filter: "blur(10px)", scale: 0.97 };
const SHOWN = { opacity: 1, filter: "blur(0px)", scale: 1 };
const FLIP = {
  scale: { type: "spring" as const, stiffness: 380, damping: 18 },
  filter: { duration: 0.22 },
  opacity: { duration: 0.16 },
};

export function IdleCard(p: {
  banner: string | null;
  exporting: boolean;
  pct: number;
  onOpenProject: () => void;
  onPreferences: () => void;
  onSettings: () => void;
  onMinimize: () => void;
  onClose: () => void;
  camRef: RefCallback<HTMLVideoElement>;
  camLive: boolean;
  cameras: DropOption[];
  camId: string;
  onCam: (id: string) => void;
  targets: DisplayInfo[];
  displayId: string;
  onTarget: (id: string) => void;
  mics: DropOption[];
  micId: string;
  onMic: (id: string) => void;
  menu: string | null;
  onMenu: (id: string) => void;
  sheet: boolean;
  onSheet: (open: boolean) => void;
  panel: string | null;
  panelBody: ReactNode;
  toggles: Toggles;
  onToggle: (key: keyof Toggles) => void;
  onRecord: () => void;
}) {
  const t = p.toggles;
  const target = p.targets
    .map((d, i) => ({ d, meta: parseTarget(d, i) }))
    .find((r) => r.d.id === p.displayId)?.meta;
  const sub = target
    ? [target.resolution?.replace("x", " by "), target.primary ? "Primary" : null].filter(Boolean).join(" · ")
    : "";
  return (
    <div className="card">
      <div className="card-head" data-tauri-drag-region>
        <span className="brand">
          <span className="brand-mark">
            <TcursorMark size={11} dotColor="var(--accent, #ef4444)" state="idle" />
          </span>
          TCursor
        </span>
        {p.banner && (
          <span className="banner" title={p.banner}>
            {"⚠"} {p.banner}
          </span>
        )}
        <span className="winctrls">
          {!p.exporting && (
            <>
              <button className="winbtn" title="Open Project" onClick={p.onOpenProject}>
                <FolderOpen />
              </button>
              <button className="winbtn" title="Preferences" onClick={p.onPreferences}>
                <Palette />
              </button>
              <button className="winbtn gear" title="Settings" onClick={p.onSettings}>
                <Gear />
              </button>
              <span className="winsep" />
            </>
          )}
          <button className="winbtn" title="Minimize" onClick={p.onMinimize}>
            <MinIcon />
          </button>
          <button className="winbtn close" title="Close" onClick={p.onClose}>
            <CloseIcon />
          </button>
        </span>
      </div>
      <AnimatePresence mode="wait" initial={false}>
        <motion.div
          key={p.panel ?? (p.sheet ? "sheet" : "card")}
          className="card-body"
          initial={FROSTED}
          animate={SHOWN}
          exit={{ ...FROSTED, transition: { duration: 0.12 } }}
          transition={FLIP}
        >
          {p.panel ? (
            p.panelBody
          ) : p.sheet ? (
            <TargetSheet
              targets={p.targets}
              value={p.displayId}
              onBack={() => p.onSheet(false)}
              onPick={(id) => {
                p.onTarget(id);
                p.onSheet(false);
              }}
            />
          ) : (
            <>
              <CamTile camRef={p.camRef} camOn={t.camOn} camLive={p.camLive} shape="wide" />
              <Dropdown
                row
                icon={<Camera />}
                value={p.camId}
                options={p.cameras}
                open={p.menu === "cam"}
                onToggle={() => p.onMenu("cam")}
                onPick={p.onCam}
              />
              <button
                type="button"
                className="dd-row"
                title="Choose what to record"
                onClick={() => p.onSheet(true)}
              >
                <span className="ico">
                  <Monitor />
                </span>
                <span className="dd-main">
                  <span className="dd-label">{target?.title ?? "-"}</span>
                  {sub && <span className="dd-sub">{sub}</span>}
                </span>
                <span className="chev right">
                  <Chevron />
                </span>
              </button>
              <Dropdown
                row
                icon={<Mic />}
                value={p.micId}
                options={p.mics}
                open={p.menu === "mic"}
                onToggle={() => p.onMenu("mic")}
                onPick={p.onMic}
              />
              <SourceToggles toggles={t} onToggle={p.onToggle} />
              <RecordButton
                disabled={p.exporting}
                onClick={p.onRecord}
                label={p.exporting ? `Exporting… ${p.pct}%` : "Record"}
              />
            </>
          )}
        </motion.div>
      </AnimatePresence>
    </div>
  );
}
