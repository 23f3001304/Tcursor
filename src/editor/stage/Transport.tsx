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

export function Transport({
  timeMs,
  dur,
  playing,
  onPlay,
  onSeek,
  onAddZoom,
  onAutoedit,
  onSplit,
  onTrim,
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
  onTrim: () => void;
  quality: number;
  onQuality: () => void;
  muted: boolean;
  onMute: () => void;
}) {
  const [volume, setVolume] = useState(100);
  const [showVolumeSlider, setShowVolumeSlider] = useState(false);
  const [aspectRatio, setAspectRatio] = useState<"16:9" | "9:16" | "1:1" | "4:3">("16:9");

  const cycleAspect = () => {
    setAspectRatio((prev) => {
      if (prev === "16:9") return "9:16";
      if (prev === "9:16") return "1:1";
      if (prev === "1:1") return "4:3";
      return "16:9";
    });
  };

  return (
    <div className="e-transport">
      {/* Left: Trim pill + divider + timeline tools */}
      <div className="e-tgroup">
        <button onClick={onTrim} className="e-tbtn" title="Trim clip range">
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
        <button className="e-chip" title="Change aspect ratio" onClick={cycleAspect}>{aspectRatio}</button>
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
