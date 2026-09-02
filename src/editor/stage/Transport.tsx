import { memo, useState } from "react";
import { motion, AnimatePresence } from "motion/react";
import {
  IconZoomIn,
  IconWand,
  IconPlayerSkipBack,
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerSkipForward,
  IconVolume,
  IconVolumeOff,
  IconArrowBarToLeft,
  IconArrowBarToRight,
  IconX,
} from "@tabler/icons-react";
import { fmt, fmtPrecise } from "../timeline/time";
import { Slider } from "../controls/Controls";
import type { Aspect } from "../../lib/edit";

/** Cycle order + short chip labels for the aspect selector - mirrors the Rust `Aspect` enum.
 *  Exported: StageToolbar's aspect quick-toggle cycles the exact same sequence, so both controls
 *  agree on "next" and never drift apart into two aspect cycles. */
export const ASPECT_ORDER: Aspect[] = ["source", "wide_16x9", "vertical_9x16", "square_1x1", "classic_4x3"];
export const ASPECT_LABEL: Record<Aspect, string> = {
  source: "Source", wide_16x9: "16:9", vertical_9x16: "9:16", square_1x1: "1:1", classic_4x3: "4:3",
};
/** The tap/hover spring the zoom/wand buttons use. */
const TAP_SPRING = { whileHover: { scale: 1.04 }, whileTap: { scale: 0.98 }, transition: { type: "tween" as const, duration: 0.12, ease: [0.4, 0, 0.2, 1] as const } };
/** Play's own press feedback (design/premium-pass D3) - a real spring, not a tween, so the hero
 *  button has more snap than the quiet chrome around it; the same spring `.e-export` now uses. */
const PLAY_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };
const PLAY_TAP = { scale: 0.94 };
/** The app-wide press spring (design/premium-pass D6) for the plain `.e-tg` icon buttons here
 *  that had no press feedback at all - reuses PLAY_SPRING's identical stiffness/damping, just a
 *  gentler scale than the hero Play button. (`.e-tbtn`/`.e-chip` siblings already have their own
 *  CSS hover-lift + `:active` press - deliberately left alone, see editor.css.) */
const PRESS_TAP = { scale: 0.96 };

// `React.memo`'d (render hygiene pass) - `timeMs` still ticks every frame during playback (the
// time readout genuinely needs it live), so this can't skip re-rendering ENTIRELY, but memo still
// avoids a re-render from unrelated `Editor` state (aiLog, dialogs, tab, ...) as long as the
// caller passes stable callback props (`Editor.tsx`'s `useEditorCallbacks`).
export const Transport = memo(function Transport({
  timeMs,
  dur,
  playing,
  onPlay,
  onSeek,
  onAddZoom,
  onAutoedit,
  aiRunning,
  exporting,
  trimmed,
  onTrimIn,
  onTrimOut,
  onResetTrim,
  aspect,
  onAspect,
  quality,
  onQuality,
  muted,
  onMute,
  volume,
  onVolume,
}: {
  timeMs: number;
  dur: number;
  playing: boolean;
  onPlay: () => void;
  onSeek: (ms: number) => void;
  onAddZoom: () => void;
  onAutoedit: () => void;
  aiRunning: boolean; // disables the wand while the AI director is mid-run, so a double click can't fire two interleaved reveals
  // Locks play/trim/aspect (via `locked` below) AND the wand (L3) while an export is running, so
  // nothing changes underfoot mid-render - the exporter renders from its own doc snapshot.
  exporting: boolean;
  trimmed: boolean;
  onTrimIn: () => void;
  onTrimOut: () => void;
  onResetTrim: () => void;
  aspect: Aspect;
  onAspect: (aspect: Aspect) => void;
  quality: number;
  onQuality: () => void;
  muted: boolean;
  onMute: () => void;
  volume: number; // 0..100 preview-audio volume (owned by Editor, applied to the <audio> element)
  onVolume: (v: number) => void;
}) {
  const [showVolumeSlider, setShowVolumeSlider] = useState(false);
  // Export in progress, or no clip loaded yet - play/trim/aspect are either unsafe (would change
  // what's rendering mid-export) or meaningless (nothing to play/trim/frame) in either state.
  const locked = exporting || dur <= 0;

  const cycleAspect = () => {
    const i = ASPECT_ORDER.indexOf(aspect);
    onAspect(ASPECT_ORDER[(i + 1) % ASPECT_ORDER.length]);
  };

  return (
    <div className="e-transport">
      {/* Left: trim start/end to the playhead (the timeline edge handles do the same), a reset that
          appears once trimmed, then a divider + timeline tools. */}
      <div className="e-tgroup">
        <button onClick={onTrimIn} className="e-tbtn" disabled={locked} title="Trim the start to the playhead (cut everything before it)">
          <IconArrowBarToLeft size={15} /><span>In</span>
        </button>
        <button onClick={onTrimOut} className="e-tbtn" disabled={locked} title="Trim the end to the playhead (cut everything after it)">
          <IconArrowBarToRight size={15} /><span>Out</span>
        </button>
        {trimmed && (
          <motion.button onClick={onResetTrim} className="e-tg on" disabled={locked} title="Reset the trim range"
            whileTap={locked ? undefined : PRESS_TAP} transition={PLAY_SPRING}><IconX size={15} /></motion.button>
        )}
        <div className="e-tdiv" />
        <motion.button className="e-tg" title="Add a zoom region here (Z)" onClick={onAddZoom} {...TAP_SPRING}>
          <IconZoomIn size={16} />
        </motion.button>
        <motion.button className="e-tg" title="Run the AI director to auto-edit this clip" onClick={onAutoedit} disabled={aiRunning || exporting}
          data-director-anchor="wand" {...(aiRunning || exporting ? {} : TAP_SPRING)}>
          <IconWand size={16} />
        </motion.button>
      </div>
      <div className="e-tdiv" />

      {/* Center: transport + single time readout. Play is the hero - a dedicated spring (scale
          .94 on press) instead of TAP_SPRING, plus a one-shot glow pulse every time `playing`
          flips: the span's `key` alternates with `playing`, so it remounts (fresh initial->animate)
          on every toggle rather than needing separate pulse-tracking state. */}
      <div className="e-tgroup" style={{ gap: 14 }}>
        <motion.button className="e-tg" title="Jump to the start" onClick={() => onSeek(0)}
          whileTap={PRESS_TAP} transition={PLAY_SPRING}>
          <IconPlayerSkipBack size={16} />
        </motion.button>
        <motion.button className="e-play" disabled={locked} title={playing ? "Pause (Space)" : "Play (Space)"} onClick={onPlay}
          whileTap={locked ? undefined : PLAY_TAP} transition={PLAY_SPRING}>
          <motion.span key={playing ? "on" : "off"} className="e-play-glow" aria-hidden
            initial={{ opacity: 1 }} animate={{ opacity: 0 }} transition={{ duration: 0.15 }} />
          {playing ? <IconPlayerPause size={17} fill="currentColor" /> : <IconPlayerPlay size={17} fill="currentColor" style={{ marginLeft: 2 }} />}
        </motion.button>
        <motion.button className="e-tg" title="Jump to the end" onClick={() => onSeek(dur)}
          whileTap={PRESS_TAP} transition={PLAY_SPRING}>
          <IconPlayerSkipForward size={16} />
        </motion.button>
        <span className="e-time">{fmtPrecise(timeMs)}<span className="sep"> / </span><span className="tot">{fmt(dur)}</span></span>
      </div>
      <div className="e-tdiv" />

      {/* Right: aspect + quality chips + volume */}
      <div className="e-tgroup" style={{ gap: 10 }}>
        <button className="e-chip" disabled={locked} title="Cycle the preview aspect ratio" onClick={cycleAspect}>{ASPECT_LABEL[aspect]}</button>
        <button className="e-chip" title="Cycle the preview quality" onClick={onQuality}>{quality}p</button>

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
    </div>
  );
});
