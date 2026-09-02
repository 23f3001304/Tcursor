import { useCallback, type RefObject } from "react";
import type { EditDoc, EditOp } from "../../lib/edit";
import { resolveTrim } from "../../lib/edit";

// The three trim actions the transport's In / Out / reset buttons fire. Kept out of Editor so the
// "trim to playhead" rules live in one place (and Editor stays under the line limit).
//
// `timeMsRef` (not a plain `timeMs: number`) + `useCallback` (render hygiene pass): the playhead
// changes every tick, but `dur`/`trim`/`applyOp` don't - reading the current time off a ref instead
// of closing over it directly means these three callbacks stay referentially stable across ticks,
// which `Transport` (`React.memo`'d) needs to actually skip re-rendering for them.
export function useTrimActions(
  doc: EditDoc | null,
  timeMsRef: RefObject<number>,
  dur: number,
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
) {
  const trim = doc?.trim ?? { in_ms: 0, out_ms: 0 };
  const range = resolveTrim(trim, dur);
  // Trim start to the playhead. out_ms is preserved (0 = "no out trim"). A click at/after the
  // current out is ignored so the range can never collapse to zero length.
  const onTrimIn = useCallback(() => {
    const i = Math.round(timeMsRef.current);
    if (i >= range.outMs) return;
    void applyOp({ op: "set_trim", in_ms: i, out_ms: trim.out_ms });
  }, [timeMsRef, range.outMs, trim.out_ms, applyOp]);
  // Trim end to the playhead. out_ms of 0 means "runs to the true end" (matches TrimOverlay), so a
  // click at/after the clip end clears the out trim. A click at/before the in point is ignored.
  const onTrimOut = useCallback(() => {
    const o = Math.round(timeMsRef.current);
    if (o <= range.inMs) return;
    void applyOp({ op: "set_trim", in_ms: trim.in_ms, out_ms: o >= dur ? 0 : o });
  }, [timeMsRef, range.inMs, trim.in_ms, dur, applyOp]);
  const onResetTrim = useCallback(() => { void applyOp({ op: "set_trim", in_ms: 0, out_ms: 0 }); }, [applyOp]);
  return { onTrimIn, onTrimOut, onResetTrim };
}
