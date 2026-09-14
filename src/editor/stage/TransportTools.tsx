import { useRef } from "react";
import { motion } from "motion/react";
import { IconArrowBarToLeft, IconArrowBarToRight, IconCut, IconEarOff, IconPlayerTrackNext, IconWand, IconX, IconZoomIn } from "@tabler/icons-react";
import type { EditDoc, EditOp } from "../../lib/edit";
import type { ClickSample } from "../../lib/ipcPreview";
import type { Range } from "../timeline/useRangeSelect";
import { MAGNET_RADIUS, MAGNET_STRENGTH, useMagnetic } from "../effects/useMagnetic";
import { PLAY_SPRING, PRESS_TAP, TAP_SPRING } from "./transportMotion";

/** How far past the playhead a click still counts as "the end of what I am doing now", and the
 *  block Cut and Speed fall back to when there is no click that soon. Both from the spec. */
export const CLICK_LOOKAHEAD_MS = 8000;
export const DEFAULT_SPAN_MS = 4000;

/** The clip-ms span Cut and Speed act on: the ruler's selection when there is one, else from the
 *  playhead to the next click within `CLICK_LOOKAHEAD_MS`, else a `DEFAULT_SPAN_MS` block. Clamped
 *  into `[0, dur]`. Pure, so the fallback is pinned by a test rather than by clicking the button. */
export function actionSpan(range: Range | null, nowMs: number, clicks: ClickSample[], dur: number): Range {
  if (range) return range;
  const next = clicks.reduce<number | null>((best, c) =>
    c.t > nowMs && c.t - nowMs <= CLICK_LOOKAHEAD_MS && (best === null || c.t < best) ? c.t : best, null);
  const start = Math.max(0, Math.round(nowMs));
  const end = Math.round(next ?? nowMs + DEFAULT_SPAN_MS);
  return [start, dur > 0 ? Math.min(dur, end) : end];
}

const RANGE_HINT = "Shift+drag the ruler to choose a range.";

/** The transport's left tool group, moved out of `Transport.tsx` (at the size cap): trim start/end
 *  to the playhead (the timeline edge handles do the same), a reset that appears once trimmed,
 *  then a divider and the timeline tools - cut a stretch out, run one at 2x, add a zoom, run the
 *  AI director. Cut and Speed act on the ruler's selection when there is one and clear it after,
 *  so the gesture reads as "choose, then act"; with nothing selected they fall back to
 *  `actionSpan`'s next-click window, which is what makes them usable without a gesture at all. */
export function TransportTools({ locked, trimmed, onTrimIn, onTrimOut, onResetTrim, onAddZoom, onAutoedit, aiRunning, exporting, timeMs, dur, clicks, range, setRange, onApply, onDetectSilences }: {
  locked: boolean; trimmed: boolean;
  onTrimIn: () => void; onTrimOut: () => void; onResetTrim: () => void;
  onAddZoom: () => void; onAutoedit: () => void;
  aiRunning: boolean; // disables the wand while the AI director is mid-run, so a double click can't fire two interleaved reveals
  exporting: boolean;
  timeMs: number; // clip time: where a range-less Cut or Speed starts
  dur: number;
  clicks: ClickSample[]; // the recording's clicks on clip time - the range-less fallback's lookahead
  range: Range | null;
  setRange: (r: Range | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onDetectSilences: () => void; // Remove silences (`useSilences`): scans the audio and applies the cuts as one undo step
}) {
  const act = (op: (a: number, b: number) => EditOp) => {
    const [a, b] = actionSpan(range, timeMs, clicks, dur);
    if (b > a) void onApply(op(a, b));
    setRange(null);
  };
  // Magnetic pull (`effects/useMagnetic`) on the two Trim pills - the only tools in this group
  // that are a decision rather than a toggle, and the pair Play is flanked by, so the three of
  // them lean together as the pointer crosses the bar. The x/y MotionValues go straight onto the
  // buttons (unlike Play, which needs a wrapper because its press already animates `y`): nothing
  // here claims the translate channel. `motion.button` and a `style` is ALL these two gain - the
  // press spring the look pass took off them stays off. Locked passes `strength: 0`, which makes
  // the hook a full no-op: a pill that will ignore the click does not lean toward it.
  const inRef = useRef<HTMLButtonElement>(null), outRef = useRef<HTMLButtonElement>(null);
  const pull = locked ? 0 : MAGNET_STRENGTH;
  const inMag = useMagnetic(inRef, MAGNET_RADIUS, pull), outMag = useMagnetic(outRef, MAGNET_RADIUS, pull);
  return (
    <div className="e-tgroup">
      <motion.button ref={inRef} style={{ x: inMag.x, y: inMag.y }}
        onClick={onTrimIn} className="e-tbtn" disabled={locked} title="Trim the start to the playhead (cut everything before it)">
        <IconArrowBarToLeft size={15} /><span>In</span>
      </motion.button>
      <motion.button ref={outRef} style={{ x: outMag.x, y: outMag.y }}
        onClick={onTrimOut} className="e-tbtn" disabled={locked} title="Trim the end to the playhead (cut everything after it)">
        <IconArrowBarToRight size={15} /><span>Out</span>
      </motion.button>
      {trimmed && (
        <motion.button onClick={onResetTrim} className="e-tg on" disabled={locked} title="Reset the trim range"
          whileTap={locked ? undefined : PRESS_TAP} transition={PLAY_SPRING}><IconX size={15} /></motion.button>
      )}
      <div className="e-tdiv" />
      <motion.button className="e-tg" disabled={locked} data-action="cut" onClick={() => act((a, b) => ({ op: "add_cut", start_ms: a, end_ms: b }))}
        title={`Remove the selected range from the clip. ${RANGE_HINT}`} {...(locked ? {} : TAP_SPRING)}>
        <IconCut size={16} />
      </motion.button>
      <motion.button className="e-tg" disabled={locked} data-action="speed" onClick={() => act((a, b) => ({ op: "set_speed", start_ms: a, end_ms: b, factor: 2 }))}
        title={`Play the selected range at 2x. ${RANGE_HINT}`} {...(locked ? {} : TAP_SPRING)}>
        <IconPlayerTrackNext size={16} />
      </motion.button>
      <motion.button className="e-tg" disabled={locked} data-action="silences" onClick={onDetectSilences}
        title="Remove silences: cut every stretch quieter than -35 dB for 0.7 s or longer, as one undo step" {...(locked ? {} : TAP_SPRING)}>
        <IconEarOff size={16} />
      </motion.button>
      <motion.button className="e-tg" title="Add a zoom region here (Z)" onClick={onAddZoom} {...TAP_SPRING}>
        <IconZoomIn size={16} />
      </motion.button>
      <motion.button className="e-tg" title="Run the AI director to auto-edit this clip" onClick={onAutoedit} disabled={aiRunning || exporting}
        data-director-anchor="wand" {...(aiRunning || exporting ? {} : TAP_SPRING)}>
        <IconWand size={16} />
      </motion.button>
    </div>
  );
}
