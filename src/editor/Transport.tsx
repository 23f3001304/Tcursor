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
import { fmt } from "./time";
import { Slider } from "./Controls";
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
    <div className="e-transport" style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12 }}>
      {/* Left section: Timeline tools */}
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <button
          onClick={onTrim}
          className="e-qual"
          style={{
            height: 32,
            padding: "0 12px",
            display: "flex",
            alignItems: "center",
            gap: 6,
            background: "var(--e-soft)",
            color: "var(--e-fg)",
            border: "1px solid var(--e-border)"
          }}
          title="Trim clip range"
        >
          <IconCut size={15} />
          <span>Trim</span>
        </button>
        
        <div style={{ width: 1, height: 20, background: "var(--e-border2)" }} />

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

      {/* Center section: Playback transport */}
      <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
        <span className="e-time" style={{ fontSize: 12.5, color: "var(--e-mut)" }}>{fmt(timeMs)}</span>
        <button className="e-tg" title="Rewind to start" onClick={() => onSeek(0)}>
          <IconPlayerSkipBack size={16} />
        </button>
        <button
          className="e-play"
          title={playing ? "Pause" : "Play"}
          onClick={onPlay}
          style={{
            width: 38,
            height: 38,
            borderRadius: "50%",
            background: "var(--e-fg, #fafafa)",
            color: "var(--e-bg, #09090b)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            border: "none",
            boxShadow: "0 2px 8px rgba(0,0,0,0.3)"
          }}
        >
          {playing ? <IconPlayerPause size={17} fill="currentColor" /> : <IconPlayerPlay size={17} fill="currentColor" style={{ marginLeft: 2 }} />}
        </button>
        <button className="e-tg" title="Fast-forward to end" onClick={() => onSeek(dur)}>
          <IconPlayerSkipForward size={16} />
        </button>
        <span className="e-time" style={{ fontSize: 12.5, color: "var(--e-mut)" }}>{fmt(dur)}</span>
      </div>

      {/* Right section: Quality & Volume */}
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <button
          className="e-qual"
          style={{ height: 26, fontSize: 11, padding: "0 6px", background: "var(--e-soft)", border: "1px solid var(--e-border)", color: "var(--e-fg)", borderRadius: 4, cursor: "pointer" }}
          title="Change aspect ratio"
          onClick={cycleAspect}
        >
          {aspectRatio}
        </button>
        <button
          className="e-qual"
          style={{ height: 26, fontSize: 11, padding: "0 6px", background: "var(--e-soft)", border: "1px solid var(--e-border)", color: "var(--e-fg)", borderRadius: 4, cursor: "pointer" }}
          title="Preview quality"
          onClick={onQuality}
        >
          {quality}p
        </button>


        <div 
          style={{ display: "flex", alignItems: "center", gap: 6, position: "relative" }}
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
                initial={{ width: 0, opacity: 0 }}
                animate={{ width: 80, opacity: 1 }}
                exit={{ width: 0, opacity: 0 }}
                transition={{ duration: 0.18 }}
                style={{ overflow: "hidden", display: "flex", alignItems: "center" }}
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
