import type { RefCallback } from "react";
import { AnimatePresence, motion } from "motion/react";
import { CamTile } from "./CamTile";
import { RecMeter } from "./RecMeter";
import { formatTimer } from "./formatTimer";
import { Pause, Play, Sliders, StopSquare } from "./icons";
import { IdleWave } from "../../lib/wave/ui/QuietWaves";

// The press spring the idle bar's toggles use (Hud.tsx TOGGLE_PRESS), a touch deeper for a
// round button whose whole face is the icon.
const PRESS = { whileTap: { scale: 0.9 }, transition: { type: "spring" as const, stiffness: 520, damping: 14 } };
/** The Pause/Play glyph swap: the old glyph shrinks away, the new one springs in past full size. */
const SWAP = { type: "spring" as const, stiffness: 520, damping: 14 };
const PULSE = { duration: 1.3, repeat: Infinity, ease: "easeInOut" as const };
const FILL = { type: "spring" as const, stiffness: 120, damping: 24 };

/** The bar while a take runs: one 60px pill instead of the idle bar's title bar and row (owner's
 *  rethink, 2026-09-14 - "three control heights, two hero buttons and a 579px slab over the
 *  screen being recorded"). Left to right: the webcam self-view as a round tile, the red dot and
 *  timer as the only hero, the level slot (`RecMeter`: the voice wave, or Paused), Sources and
 *  Pause as quiet icon buttons and Stop as the single red control. Sources opens the sheet under
 *  the pill (`SourcesSheet`) where the display, camera and microphone can be changed without
 *  stopping the take. A source that is off (camera, or both
 *  audio sources) is simply absent. No minimize, no close:
 *  Stop is the way out, and closing the window some other way still stops and saves through
 *  `Hud`'s close guard. The body of the pill drags the window (`data-tauri-drag-region`;
 *  `.take > :not(button)` is pointer-events none so a click anywhere but a button reaches it).
 *
 *  Saving is the same pill with the brand's idle wave, the word Saving, the percentage and a
 *  progress line that stretches to fill - the pill keeps its width across every take state, so
 *  the window never resizes between Record and the editor (`useHudWindowSize`'s `TAKE_WIDTH`).
 *
 *  Owns no state: every prop is `Hud`'s own truth. */
export function TakeBar({ paused, saving, savePct, elapsed, err, micOn, sysOn, live, read, camRef, camOn, camLive, toggle, togglePause, sources, onSources }: {
  paused: boolean; saving: boolean; savePct: number; elapsed: number; err: string | null;
  micOn: boolean; sysOn: boolean; live: boolean; read: () => { mic: number; sys: number };
  camRef: RefCallback<HTMLVideoElement>; camOn: boolean; camLive: boolean;
  toggle: () => void; togglePause: () => void;
  sources: boolean; onSources: () => void;
}) {
  if (saving) {
    return (
      <div className="take saving" data-tauri-drag-region role="status" aria-label={`Saving, ${savePct}%`}>
        <span className="take-spin"><IdleWave size={18} /></span>
        <span className="take-label">Saving</span>
        <span className="take-pct">{savePct}%</span>
        <span className="take-bar">
          <motion.span className="take-fill" initial={false} animate={{ width: `${savePct}%` }} transition={FILL} />
        </span>
      </div>
    );
  }
  // A source that is off is not in the pill at all (owner, 2026-09-14): no off-glyph webcam
  // circle, no struck mic. The window is sized for exactly what is shown (`takeWidth`).
  return (
    <div className={`take${paused ? " paused" : ""}`} data-tauri-drag-region>
      {camOn && <CamTile camRef={camRef} camOn={camOn} camLive={camLive} shape="round" />}
      <span className="take-clock">
        {/* The dot breathes while the take runs and holds still, grey, while paused - the one
            continuous motion in the pill, and it stops the moment the recording does. */}
        <motion.span className="take-dot" animate={{ opacity: paused ? 1 : [1, 0.35, 1] }}
          transition={paused ? { duration: 0.16 } : PULSE} />
        <span className="take-timer">{formatTimer(elapsed)}</span>
      </span>
      {/* The title bar's warning slot has no title bar to live in here: the message rides on a
          hover instead, the one non-button in the pill that keeps its pointer events. */}
      {err && <span className="take-warn" title={err} aria-label={err}>!</span>}
      {(micOn || sysOn) && <RecMeter live={live} read={read} paused={paused} />}
      <motion.button type="button" className={`take-btn${sources ? " on" : ""}`} title="Sources"
        aria-label="Sources" aria-expanded={sources} onClick={onSources} {...PRESS}>
        <Sliders />
      </motion.button>
      <motion.button type="button" className="take-btn" title={paused ? "Resume" : "Pause"}
        aria-label={paused ? "Resume" : "Pause"} onClick={togglePause} {...PRESS}>
        <AnimatePresence mode="wait" initial={false}>
          <motion.span key={paused ? "play" : "pause"} className="take-ico" initial={{ scale: 0.4, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }} exit={{ scale: 0.4, opacity: 0, transition: { duration: 0.08 } }} transition={SWAP}>
            {paused ? <Play /> : <Pause />}
          </motion.span>
        </AnimatePresence>
      </motion.button>
      <motion.button type="button" className="take-btn stop" title="Stop and save" aria-label="Stop and save"
        onClick={toggle} {...PRESS}>
        <StopSquare />
      </motion.button>
    </div>
  );
}
