import { useCallback, type RefObject } from "react";
import type { EditDoc, EditOp } from "../../../shared/edit";
import { resolveTrim } from "../../../shared/edit";

export function useTrimActions(
  doc: EditDoc | null,
  timeMsRef: RefObject<number>,
  dur: number,
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
) {
  const trim = doc?.trim ?? { in_ms: 0, out_ms: 0 };
  const range = resolveTrim(trim, dur);
  const onTrimIn = useCallback(() => {
    const i = Math.round(timeMsRef.current);
    if (i >= range.outMs) return;
    void applyOp({ op: "set_trim", in_ms: i, out_ms: trim.out_ms });
  }, [timeMsRef, range.outMs, trim.out_ms, applyOp]);
  const onTrimOut = useCallback(() => {
    const o = Math.round(timeMsRef.current);
    if (o <= range.inMs) return;
    void applyOp({ op: "set_trim", in_ms: trim.in_ms, out_ms: o >= dur ? 0 : o });
  }, [timeMsRef, range.inMs, trim.in_ms, dur, applyOp]);
  const onResetTrim = useCallback(() => {
    void applyOp({ op: "set_trim", in_ms: 0, out_ms: 0 });
  }, [applyOp]);
  return { onTrimIn, onTrimOut, onResetTrim };
}
