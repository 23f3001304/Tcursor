import { motion } from "motion/react";
import { formatTimer } from "./formatTimer";

/** The recording row's right-hand cluster - timer, "Paused" chip, Pause/Resume, and the hero
 *  Record/Stop/Saving button. Split out of `Hud.tsx` (200-line cap) alongside `CamTile`/`RecMeter`.
 *
 *  Owns no state - purely a rendering of the truth `Hud`'s `useRecordingFlow`/`useRecordingTimer`
 *  already computed. The `recording`-gated pieces (timer/chip/Pause) render only while actually
 *  recording, exactly mirroring `Hud.tsx`'s own prior inline JSX; the hero button is the only
 *  piece that spans all three bar states (idle "Record", recording "Stop", saving "Saving…",
 *  disabled). */
export function RecordingControls({ recording, paused, elapsed, saving, exporting, toggle, togglePause }: {
  recording: boolean; paused: boolean; elapsed: number; saving: boolean; exporting: boolean;
  toggle: () => void; togglePause: () => void;
}) {
  return (
    <>
      {recording && <span className={`timer ${paused ? "paused" : ""}`}>{formatTimer(elapsed)}</span>}
      {recording && paused && (
        <motion.span className="paused-chip" animate={{ opacity: [0.55, 1, 0.55] }}
          transition={{ duration: 1.6, repeat: Infinity, ease: "easeInOut" }}>Paused</motion.span>
      )}
      {recording && <button className="btn" onClick={togglePause}>{paused ? "Resume" : "Pause"}</button>}
      <motion.button className={`btn rec ${recording ? "is-rec" : ""}`} onClick={toggle} disabled={exporting || saving}
        whileTap={{ scale: 0.92 }} transition={{ type: "spring", stiffness: 500, damping: 30 }}>
        <span className="dot" />{saving ? "Saving…" : recording ? "Stop" : "Record"}
      </motion.button>
    </>
  );
}
