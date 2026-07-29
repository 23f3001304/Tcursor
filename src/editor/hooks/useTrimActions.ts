import type { EditDoc, EditOp } from "../../lib/edit";
import { resolveTrim } from "../../lib/edit";

// The three trim actions the transport's In / Out / reset buttons fire. Kept out of Editor so the
// "trim to playhead" rules live in one place (and Editor stays under the line limit). Returns fresh
// closures each render over the current playhead/duration - no React state of its own.
export function useTrimActions(
  doc: EditDoc | null,
  timeMs: number,
  dur: number,
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
) {
  const trim = doc?.trim ?? { in_ms: 0, out_ms: 0 };
  const range = resolveTrim(trim, dur);
  return {
    // Trim start to the playhead. out_ms is preserved (0 = "no out trim"). A click at/after the
    // current out is ignored so the range can never collapse to zero length.
    onTrimIn: () => {
      const i = Math.round(timeMs);
      if (i >= range.outMs) return;
      void applyOp({ op: "set_trim", in_ms: i, out_ms: trim.out_ms });
    },
    // Trim end to the playhead. out_ms of 0 means "runs to the true end" (matches TrimOverlay), so a
    // click at/after the clip end clears the out trim. A click at/before the in point is ignored.
    onTrimOut: () => {
      const o = Math.round(timeMs);
      if (o <= range.inMs) return;
      void applyOp({ op: "set_trim", in_ms: trim.in_ms, out_ms: o >= dur ? 0 : o });
    },
    onResetTrim: () => { void applyOp({ op: "set_trim", in_ms: 0, out_ms: 0 }); },
  };
}
