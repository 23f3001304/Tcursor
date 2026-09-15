import { useState } from "react";
import { motion, AnimatePresence } from "motion/react";
import { IconVolume, IconVolumeOff } from "@tabler/icons-react";
import { Slider } from "../../controls/Controls";
import type { Aspect } from "../../../shared/edit";
import { PLAY_SPRING, PRESS_TAP } from "./transportMotion";
import { ViewPicker } from "./ViewPicker";

const ASPECT_ORDER: Aspect[] = ["source", "wide_16x9", "vertical_9x16", "square_1x1", "classic_4x3"];
const ASPECT_LABEL: Record<Aspect, string> = {
  source: "Source",
  wide_16x9: "16:9",
  vertical_9x16: "9:16",
  square_1x1: "1:1",
  classic_4x3: "4:3",
};

export function OutputGroup({
  locked,
  aspect,
  onAspect,
  quality,
  onQuality,
  muted,
  onMute,
  volume,
  onVolume,
}: {
  locked: boolean;
  aspect: Aspect;
  onAspect: (aspect: Aspect) => void;
  quality: number;
  onQuality: () => void;
  muted: boolean;
  onMute: () => void;
  volume: number;
  onVolume: (v: number) => void;
}) {
  const [showVolumeSlider, setShowVolumeSlider] = useState(false);
  const cycleAspect = () => {
    const i = ASPECT_ORDER.indexOf(aspect);
    onAspect(ASPECT_ORDER[(i + 1) % ASPECT_ORDER.length]);
  };

  return (
    <div className="e-tgroup" style={{ gap: 10 }}>
      <button
        className="e-chip"
        disabled={locked}
        title="Cycle the preview aspect ratio"
        onClick={cycleAspect}
      >
        {ASPECT_LABEL[aspect]}
      </button>
      <button className="e-chip" title="Cycle the preview quality" onClick={onQuality}>
        {quality}p
      </button>
      <ViewPicker />

      <div
        className="e-volwrap"
        onMouseEnter={() => setShowVolumeSlider(true)}
        onMouseLeave={() => setShowVolumeSlider(false)}
      >
        <motion.button
          className={`e-tg ${muted ? "on" : ""}`}
          onClick={onMute}
          title={muted ? "Unmute audio" : "Mute audio"}
          whileTap={PRESS_TAP}
          transition={PLAY_SPRING}
        >
          {muted || volume === 0 ? <IconVolumeOff size={16} /> : <IconVolume size={16} />}
        </motion.button>

        <AnimatePresence>
          {showVolumeSlider && (
            <motion.div
              className="e-volflyout"
              initial={{ opacity: 0, x: 6 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 6 }}
              transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
            >
              <div style={{ width: 70, marginLeft: 6 }}>
                <Slider
                  min={0}
                  max={100}
                  step={1}
                  value={muted ? 0 : volume}
                  onChange={(v) => {
                    onVolume(v);
                    if (muted && v > 0) onMute();
                  }}
                  accentColor="var(--e-fg)"
                  ariaLabel="Volume"
                />
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
