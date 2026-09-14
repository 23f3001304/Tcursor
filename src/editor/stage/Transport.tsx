import { memo, useRef, useState } from "react";
import { motion, AnimatePresence } from "motion/react";
import {
  IconPlayerSkipBack,
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerSkipForward,
  IconVolume,
  IconVolumeOff,
} from "@tabler/icons-react";
import { fmt, fmtPrecise } from "../timeline/time";
import { Slider } from "../controls/Controls";
import type { Aspect, EditDoc, EditOp } from "../../lib/edit";
import type { ClickSample } from "../../lib/ipcPreview";
import type { Range } from "../timeline/useRangeSelect";
import { MAGNET_RADIUS, MAGNET_STRENGTH, useMagnetic } from "../effects/useMagnetic";
import { PLAY_SPRING, PLAY_TAP, PRESS_TAP } from "./transportMotion";
import { TransportTools } from "./TransportTools";
import { ViewPicker } from "./ViewPicker";

/** Cycle order + short chip labels for the aspect selector - mirrors the Rust `Aspect` enum. The
 *  chip below is the editor's ONE aspect control now: the floating stage toolbar that carried a
 *  second copy of this cycle is gone, so these no longer need to be exported. */
const ASPECT_ORDER: Aspect[] = ["source", "wide_16x9", "vertical_9x16", "square_1x1", "classic_4x3"];
const ASPECT_LABEL: Record<Aspect, string> = {
  source: "Source", wide_16x9: "16:9", vertical_9x16: "9:16", square_1x1: "1:1", classic_4x3: "4:3",
};

// `React.memo`'d (render hygiene pass) - `timeMs` still ticks every frame during playback (the
// time readout genuinely needs it live), so this can't skip re-rendering ENTIRELY, but memo still
// avoids a re-render from unrelated `Editor` state (aiLog, dialogs, tab, ...) as long as the
// caller passes stable callback props (`Editor.tsx`'s `useEditorCallbacks`).
export const Transport = memo(function Transport({
  timeMs,
  dur,
  outTimeMs,
  outDur,
  plain,
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
  clicks,
  range,
  setRange,
  onApply,
  onDetectSilences,
}: {
  timeMs: number; // clip time (the raw recording's clock)
  dur: number; // the raw clip's length: the seek range
  outTimeMs: number; // output time: what the viewer sees, cuts skipped and speed applied
  outDur: number; // the exported length
  plain: boolean; // no cuts and no speed spans: the two clocks agree, so only one is shown
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
  // The three inputs the Cut / Speed tools need, threaded straight to `TransportTools`: the
  // recording's clicks (the range-less fallback's lookahead), the ruler's selection, and the op sink.
  clicks: ClickSample[];
  range: Range | null;
  setRange: (r: Range | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onDetectSilences: () => void;
}) {
  const [showVolumeSlider, setShowVolumeSlider] = useState(false);
  // Export in progress, or no clip loaded yet - play/trim/aspect are either unsafe (would change
  // what's rendering mid-export) or meaningless (nothing to play/trim/frame) in either state.
  const locked = exporting || dur <= 0;

  // Play leans toward a near pointer (`effects/useMagnetic`), like the Trim pills beside it.
  // `PLAY_TAP` is scale-only, so the translate channel is free; `locked` -> strength 0 -> no-op.
  const playRef = useRef<HTMLButtonElement>(null);
  const playMag = useMagnetic(playRef, MAGNET_RADIUS, locked ? 0 : MAGNET_STRENGTH);

  const cycleAspect = () => {
    const i = ASPECT_ORDER.indexOf(aspect);
    onAspect(ASPECT_ORDER[(i + 1) % ASPECT_ORDER.length]);
  };

  return (
    <div className="e-transport">
      <TransportTools locked={locked} trimmed={trimmed} onTrimIn={onTrimIn} onTrimOut={onTrimOut} onResetTrim={onResetTrim}
        onAddZoom={onAddZoom} onAutoedit={onAutoedit} aiRunning={aiRunning} exporting={exporting}
        timeMs={timeMs} dur={dur} clicks={clicks} range={range} setRange={setRange} onApply={onApply} onDetectSilences={onDetectSilences} />
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
        <motion.button ref={playRef} style={{ x: playMag.x, y: playMag.y }}
          className="e-play" disabled={locked} title={playing ? "Pause (Space)" : "Play (Space)"} onClick={onPlay}
          whileTap={locked ? undefined : PLAY_TAP} transition={PLAY_SPRING}>
          <motion.span key={playing ? "on" : "off"} className="e-play-glow" aria-hidden
            initial={{ opacity: 1 }} animate={{ opacity: 0 }} transition={{ duration: 0.15 }} />
          {playing ? <IconPlayerPause size={17} fill="currentColor" /> : <IconPlayerPlay size={17} fill="currentColor" />}
        </motion.button>
        <motion.button className="e-tg" title="Jump to the end" onClick={() => onSeek(dur)}
          whileTap={PRESS_TAP} transition={PLAY_SPRING}>
          <IconPlayerSkipForward size={16} />
        </motion.button>
        {/* Output time large (what the viewer will see); clip time as fine print only when a cut or a
            speed span makes the two clocks differ - every lane on the timeline still sits on clip time. */}
        <span className="e-time">{fmtPrecise(outTimeMs)}<span className="sep"> / </span><span className="tot">{fmt(outDur)}</span>
          {!plain && <span className="e-time-clip" title="Clip time, the raw recording's clock">{fmtPrecise(timeMs)}</span>}</span>
      </div>
      <div className="e-tdiv" />

      {/* Right: aspect + quality chips, how the frame sits in the stage, then volume. */}
      <div className="e-tgroup" style={{ gap: 10 }}>
        <button className="e-chip" disabled={locked} title="Cycle the preview aspect ratio" onClick={cycleAspect}>{ASPECT_LABEL[aspect]}</button>
        <button className="e-chip" title="Cycle the preview quality" onClick={onQuality}>{quality}p</button>
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
                // Slides out of the button leftward (the flyout opens inward now - see
                // `.e-volflyout` in stage.css for why it cannot open past the bar's right edge).
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
    </div>
  );
});
