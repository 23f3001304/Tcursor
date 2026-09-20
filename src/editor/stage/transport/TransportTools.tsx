import { useRef } from "react";
import { motion } from "motion/react";
import {
  IconArrowBarToLeft,
  IconArrowBarToRight,
  IconCut,
  IconEarOff,
  IconPlayerTrackNext,
  IconScissors,
  IconTypography,
  IconWand,
  IconX,
  IconZoomIn,
} from "@tabler/icons-react";
import type { EditDoc, EditOp } from "../../../shared/edit";
import type { ClickSample } from "../../../shared/ipc";
import type { Range } from "../../timeline/useRangeSelect";
import { MAGNET_RADIUS, MAGNET_STRENGTH, useMagnetic } from "../../effects/useMagnetic";
import { PLAY_SPRING, PRESS_TAP, TAP_SPRING } from "./transportMotion";

export const CLICK_LOOKAHEAD_MS = 8000;
export const DEFAULT_SPAN_MS = 4000;

export function actionSpan(range: Range | null, nowMs: number, clicks: ClickSample[], dur: number): Range {
  if (range) return range;
  const next = clicks.reduce<number | null>(
    (best, c) =>
      c.t > nowMs && c.t - nowMs <= CLICK_LOOKAHEAD_MS && (best === null || c.t < best) ? c.t : best,
    null,
  );
  const start = Math.max(0, Math.round(nowMs));
  const end = Math.round(next ?? nowMs + DEFAULT_SPAN_MS);
  return [start, dur > 0 ? Math.min(dur, end) : end];
}

const RANGE_HINT = "Shift+drag the ruler to choose a range.";

export function TransportTools({
  locked,
  trimmed,
  onTrimIn,
  onTrimOut,
  onResetTrim,
  onAddZoom,
  onSplit,
  onAddText,
  onAutoedit,
  aiRunning,
  exporting,
  timeMs,
  dur,
  clicks,
  range,
  setRange,
  onApply,
  onDetectSilences,
}: {
  locked: boolean;
  trimmed: boolean;
  onTrimIn: () => void;
  onTrimOut: () => void;
  onResetTrim: () => void;
  onAddZoom: () => void;
  onSplit: () => void;
  onAddText: () => void;
  onAutoedit: () => void;
  aiRunning: boolean;
  exporting: boolean;
  timeMs: number;
  dur: number;
  clicks: ClickSample[];
  range: Range | null;
  setRange: (r: Range | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onDetectSilences: () => void;
}) {
  const act = (op: (a: number, b: number) => EditOp) => {
    const [a, b] = actionSpan(range, timeMs, clicks, dur);
    if (b > a) void onApply(op(a, b));
    setRange(null);
  };
  const inRef = useRef<HTMLButtonElement>(null),
    outRef = useRef<HTMLButtonElement>(null);
  const pull = locked ? 0 : MAGNET_STRENGTH;
  const inMag = useMagnetic(inRef, MAGNET_RADIUS, pull),
    outMag = useMagnetic(outRef, MAGNET_RADIUS, pull);
  return (
    <div className="e-tgroup">
      <motion.button
        ref={inRef}
        style={{ x: inMag.x, y: inMag.y }}
        onClick={onTrimIn}
        className="e-tbtn"
        disabled={locked}
        title="Trim the start to the playhead (cut everything before it)"
      >
        <IconArrowBarToLeft size={15} />
        <span>In</span>
      </motion.button>
      <motion.button
        ref={outRef}
        style={{ x: outMag.x, y: outMag.y }}
        onClick={onTrimOut}
        className="e-tbtn"
        disabled={locked}
        title="Trim the end to the playhead (cut everything after it)"
      >
        <IconArrowBarToRight size={15} />
        <span>Out</span>
      </motion.button>
      {trimmed && (
        <motion.button
          onClick={onResetTrim}
          className="e-tg on"
          disabled={locked}
          title="Reset the trim range"
          whileTap={locked ? undefined : PRESS_TAP}
          transition={PLAY_SPRING}
        >
          <IconX size={15} />
        </motion.button>
      )}
      <div className="e-tdiv" />
      <motion.button
        className="e-tg"
        disabled={locked}
        data-action="cut"
        onClick={() => act((a, b) => ({ op: "add_cut", start_ms: a, end_ms: b }))}
        title={`Remove the selected range from the clip. ${RANGE_HINT}`}
        {...(locked ? {} : TAP_SPRING)}
      >
        <IconCut size={16} />
      </motion.button>
      <motion.button
        className="e-tg"
        disabled={locked}
        data-action="speed"
        onClick={() => act((a, b) => ({ op: "set_speed", start_ms: a, end_ms: b, factor: 2 }))}
        title={`Play the selected range at 2x. ${RANGE_HINT}`}
        {...(locked ? {} : TAP_SPRING)}
      >
        <IconPlayerTrackNext size={16} />
      </motion.button>
      <motion.button
        className="e-tg"
        disabled={locked}
        data-action="silences"
        onClick={onDetectSilences}
        title="Remove silences: cut every stretch quieter than -35 dB for 0.7 s or longer, as one undo step"
        {...(locked ? {} : TAP_SPRING)}
      >
        <IconEarOff size={16} />
      </motion.button>
      <motion.button className="e-tg" title="Add a zoom region here (Z)" onClick={onAddZoom} {...TAP_SPRING}>
        <IconZoomIn size={16} />
      </motion.button>
      <motion.button
        className="e-tg"
        disabled={locked}
        data-action="split"
        title="Split at the playhead (B)"
        onClick={onSplit}
        {...(locked ? {} : TAP_SPRING)}
      >
        <IconScissors size={16} />
      </motion.button>
      <motion.button
        className="e-tg"
        title="Add a title, lower third, stat or callout here (T)"
        onClick={onAddText}
        {...TAP_SPRING}
      >
        <IconTypography size={16} />
      </motion.button>
      <motion.button
        className="e-tg"
        title="Run the AI director to auto-edit this clip"
        onClick={onAutoedit}
        disabled={aiRunning || exporting}
        data-director-anchor="wand"
        {...(aiRunning || exporting ? {} : TAP_SPRING)}
      >
        <IconWand size={16} />
      </motion.button>
    </div>
  );
}
