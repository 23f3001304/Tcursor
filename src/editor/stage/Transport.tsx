import { useState } from "react";
import {
  IconZoomIn,
  IconScissors,
  IconWand,
  IconPlayerSkipBack,
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerSkipForward,
  IconVolume,
  IconVolumeOff,
  IconCut
} from "@tabler/icons-react";
import { fmt } from "../timeline/time";
import { Slider } from "../controls/Controls";
import { motion, AnimatePresence } from "motion/react";
import type { Aspect } from "../../lib/edit";

/** Cycle order + short chip labels for the aspect selector - mirrors the Rust `Aspect` enum. */
const ASPECT_ORDER: Aspect[] = ["source", "wide_16x9", "vertical_9x16", "square_1x1", "classic_4x3"];
const ASPECT_LABEL: Record<Aspect, string> = {
  source: "Source", wide_16x9: "16:9", vertical_9x16: "9:16", square_1x1: "1:1", classic_4x3: "4:3",
};

export function Transport({
  timeMs,
  dur,
  playing,
  onPlay,
  onSeek,
  onAddZoom,
  onAutoedit,
  onSplit,
  trimmed,
  onResetTrim,
  aspect,
  onAspect,
  quality,
  onQuality,
  muted,
  onMute,
}: {
  timeMs: number;
  dur: number;
  playing: boolean;
  onPlay: () => void;
  onSeek: (ms: number) => void;
  onAddZoom: () => void;
  onAutoedit: () => void;
  onSplit: () => void;
  trimmed: boolean;
  onResetTrim: () => void;
  aspect: Aspect;
  onAspect: (aspect: Aspect) => void;
  quality: number;
  onQuality: () => void;
  muted: boolean;
  onMute: () => void;
}) {
  const [volume, setVolume] = useState(100);
  const [showVolumeSlider, setShowVolumeSlider] = useState(false);

  const cycleAspect = () => {
    const i = ASPECT_ORDER.indexOf(aspect);
    onAspect(ASPECT_ORDER[(i + 1) % ASPECT_ORDER.length]);
  };

  return (
    <div className="e-transport">
      {/* Left: Trim pill (drag the timeline's edge handles to trim; this resets it) + divider + timeline tools */}
      <div className="e-tgroup">
        <button onClick={onResetTrim} className={`e-tbtn${trimmed ? " on" : ""}`}
          title={trimmed ? "Reset trim range" : "Drag the timeline's edge handles to trim"} disabled={!trimmed}>
          <IconCut size={15} />
          <span>Trim</span>
        </button>
        <div className="e-tdiv" />
        <button className="e-tg" title="Add zoom region" onClick={onAddZoom}>
          <IconZoomIn size={16} />
        </button>
        <button className="e-tg" title="Run AI Auto-director" onClick={onAutoedit}>
          <IconWand size={16} />
        </button>
        <button className="e-tg" title="Split clip at playhead" onClick={onSplit}>
          <IconScissors size={16} />
        </button>
      </div>

      {/* Center: transport + single time readout */}
      <div className="e-tgroup" style={{ gap: 14 }}>
        <button className="e-tg" title="Rewind to start" onClick={() => onSeek(0)}>
          <IconPlayerSkipBack size={16} />
        </button>
        <motion.button className="e-play" title={playing ? "Pause" : "Play"} onClick={onPlay}
          whileHover={{ scale: 1.04 }} whileTap={{ scale: 0.98 }} transition={{ type: "tween", duration: 0.12, ease: [0.4, 0, 0.2, 1] }}>
          {playing ? <IconPlayerPause size={17} fill="currentColor" /> : <IconPlayerPlay size={17} fill="currentColor" style={{ marginLeft: 2 }} />}
        </motion.button>
        <button className="e-tg" title="Fast-forward to end" onClick={() => onSeek(dur)}>
          <IconPlayerSkipForward size={16} />
        </button>
        <span className="e-time">{fmt(timeMs)}<span className="sep"> / </span><span className="tot">{fmt(dur)}</span></span>
      </div>

      {/* Right: aspect + quality chips + volume */}
      <div className="e-tgroup" style={{ gap: 10 }}>
        <button className="e-chip" title="Change aspect ratio" onClick={cycleAspect}>{ASPECT_LABEL[aspect]}</button>
        <button className="e-chip" title="Preview quality" onClick={onQuality}>{quality}p</button>

        <div
          className="e-volwrap"
          onMouseEnter={() => setShowVolumeSlider(true)}
          onMouseLeave={() => setShowVolumeSlider(false)}
        >
          <button
            className={`e-tg ${muted ? "on" : ""}`}
            onClick={onMute}
            title={muted ? "Unmute audio" : "Mute audio"}
          >
            {muted || volume === 0 ? <IconVolumeOff size={16} /> : <IconVolume size={16} />}
          </button>

          <AnimatePresence>
            {showVolumeSlider && (
              <motion.div
                className="e-volflyout"
                initial={{ opacity: 0, x: -6 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -6 }}
                transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
              >
                <div style={{ width: 70, marginLeft: 4 }}>
                  <Slider
                    min={0}
                    max={100}
                    step={1}
                    value={muted ? 0 : volume}
                    onChange={(v) => {
                      setVolume(v);
                      if (muted && v > 0) onMute();
                    }}
                    accentColor="var(--e-fg)"
                  />
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>
    </div>
  );
}
