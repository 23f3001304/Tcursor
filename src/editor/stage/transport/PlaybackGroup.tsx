import { useRef } from "react";
import { motion } from "motion/react";
import {
  IconPlayerSkipBack,
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerSkipForward,
} from "@tabler/icons-react";
import { fmt, fmtPrecise } from "../../timeline/model/time";
import { MAGNET_RADIUS, MAGNET_STRENGTH, useMagnetic } from "../../effects/useMagnetic";
import { PLAY_SPRING, PLAY_TAP, PRESS_TAP } from "./transportMotion";

export function PlaybackGroup({
  timeMs,
  dur,
  outTimeMs,
  outDur,
  plain,
  playing,
  locked,
  onPlay,
  onSeek,
}: {
  timeMs: number;
  dur: number;
  outTimeMs: number;
  outDur: number;
  plain: boolean;
  playing: boolean;
  locked: boolean;
  onPlay: () => void;
  onSeek: (ms: number) => void;
}) {
  const playRef = useRef<HTMLButtonElement>(null);
  const playMag = useMagnetic(playRef, MAGNET_RADIUS, locked ? 0 : MAGNET_STRENGTH);

  return (
    <div className="e-tgroup" style={{ gap: 14 }}>
      <motion.button
        className="e-tg"
        title="Jump to the start"
        onClick={() => onSeek(0)}
        whileTap={PRESS_TAP}
        transition={PLAY_SPRING}
      >
        <IconPlayerSkipBack size={16} />
      </motion.button>
      <motion.button
        ref={playRef}
        style={{ x: playMag.x, y: playMag.y }}
        className="e-play"
        disabled={locked}
        title={playing ? "Pause (Space)" : "Play (Space)"}
        onClick={onPlay}
        whileTap={locked ? undefined : PLAY_TAP}
        transition={PLAY_SPRING}
      >
        <motion.span
          key={playing ? "on" : "off"}
          className="e-play-glow"
          aria-hidden
          initial={{ opacity: 1 }}
          animate={{ opacity: 0 }}
          transition={{ duration: 0.15 }}
        />
        {playing ? (
          <IconPlayerPause size={17} fill="currentColor" />
        ) : (
          <IconPlayerPlay size={17} fill="currentColor" />
        )}
      </motion.button>
      <motion.button
        className="e-tg"
        title="Jump to the end"
        onClick={() => onSeek(dur)}
        whileTap={PRESS_TAP}
        transition={PLAY_SPRING}
      >
        <IconPlayerSkipForward size={16} />
      </motion.button>
      <span className="e-time">
        {fmtPrecise(outTimeMs)}
        <span className="sep"> / </span>
        <span className="tot">{fmt(outDur)}</span>
        {!plain && (
          <span className="e-time-clip" title="Clip time, the raw recording's clock">
            {fmtPrecise(timeMs)}
          </span>
        )}
      </span>
    </div>
  );
}
