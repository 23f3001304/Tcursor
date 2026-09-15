import type { RefCallback } from "react";
import { AnimatePresence, motion } from "motion/react";
import { CamTile } from "./CamTile";
import { RecMeter } from "./RecMeter";
import { formatTimer } from "./formatTimer";
import { Pause, Play, Sliders, StopSquare } from "./icons";
import { IdleWave } from "../../shared/wave/ui/QuietWaves";

const PRESS = {
  whileTap: { scale: 0.9 },
  transition: { type: "spring" as const, stiffness: 520, damping: 14 },
};

const SWAP = { type: "spring" as const, stiffness: 520, damping: 14 };
const PULSE = { duration: 1.3, repeat: Infinity, ease: "easeInOut" as const };
const FILL = { type: "spring" as const, stiffness: 120, damping: 24 };

export function TakeBar({
  paused,
  saving,
  savePct,
  elapsed,
  err,
  micOn,
  sysOn,
  live,
  read,
  camRef,
  camOn,
  camLive,
  toggle,
  togglePause,
  sources,
  onSources,
}: {
  paused: boolean;
  saving: boolean;
  savePct: number;
  elapsed: number;
  err: string | null;
  micOn: boolean;
  sysOn: boolean;
  live: boolean;
  read: () => { mic: number; sys: number };
  camRef: RefCallback<HTMLVideoElement>;
  camOn: boolean;
  camLive: boolean;
  toggle: () => void;
  togglePause: () => void;
  sources: boolean;
  onSources: () => void;
}) {
  if (saving) {
    return (
      <div className="take saving" data-tauri-drag-region role="status" aria-label={`Saving, ${savePct}%`}>
        <span className="take-spin">
          <IdleWave size={18} />
        </span>
        <span className="take-label">Saving</span>
        <span className="take-pct">{savePct}%</span>
        <span className="take-bar">
          <motion.span
            className="take-fill"
            initial={false}
            animate={{ width: `${savePct}%` }}
            transition={FILL}
          />
        </span>
      </div>
    );
  }
  return (
    <div className={`take${paused ? " paused" : ""}`} data-tauri-drag-region>
      {camOn && <CamTile camRef={camRef} camOn={camOn} camLive={camLive} shape="round" />}
      <span className="take-clock">
        <motion.span
          className="take-dot"
          animate={{ opacity: paused ? 1 : [1, 0.35, 1] }}
          transition={paused ? { duration: 0.16 } : PULSE}
        />
        <span className="take-timer">{formatTimer(elapsed)}</span>
      </span>
      {err && (
        <span className="take-warn" title={err} aria-label={err}>
          !
        </span>
      )}
      {(micOn || sysOn) && <RecMeter live={live} read={read} paused={paused} />}
      <motion.button
        type="button"
        className={`take-btn${sources ? " on" : ""}`}
        title="Sources"
        aria-label="Sources"
        aria-expanded={sources}
        onClick={onSources}
        {...PRESS}
      >
        <Sliders />
      </motion.button>
      <motion.button
        type="button"
        className="take-btn"
        title={paused ? "Resume" : "Pause"}
        aria-label={paused ? "Resume" : "Pause"}
        onClick={togglePause}
        {...PRESS}
      >
        <AnimatePresence mode="wait" initial={false}>
          <motion.span
            key={paused ? "play" : "pause"}
            className="take-ico"
            initial={{ scale: 0.4, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            exit={{ scale: 0.4, opacity: 0, transition: { duration: 0.08 } }}
            transition={SWAP}
          >
            {paused ? <Play /> : <Pause />}
          </motion.span>
        </AnimatePresence>
      </motion.button>
      <motion.button
        type="button"
        className="take-btn stop"
        title="Stop and save"
        aria-label="Stop and save"
        onClick={toggle}
        {...PRESS}
      >
        <StopSquare />
      </motion.button>
    </div>
  );
}
