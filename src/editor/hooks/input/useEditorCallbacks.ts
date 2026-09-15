import { useCallback, useRef, type RefObject } from "react";
import type { Aspect, EditDoc, EditOp } from "../../../shared/edit";

export function useEditorCallbacks(args: {
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  sel: string | null;
  dur: number;
  timeMsRef: RefObject<number>;
  trimRangeRef: RefObject<{ inMs: number; outMs: number }>;
  setTimeMs: (ms: number) => void;
  setPlaying: (fn: boolean | ((p: boolean) => boolean)) => void;
  setAimOn: (v: boolean) => void;
  setQuality: (fn: (q: number) => number) => void;
  setMuted: (fn: (m: boolean) => boolean) => void;
  requestMoveMode: (want: boolean) => void;
}) {
  const {
    applyOp,
    dur,
    timeMsRef,
    trimRangeRef,
    setTimeMs,
    setPlaying,
    setAimOn,
    setQuality,
    setMuted,
    requestMoveMode,
  } = args;
  const selRef = useRef(args.sel);
  selRef.current = args.sel;

  const onTime = useCallback(
    (ms: number) => {
      const { outMs } = trimRangeRef.current ?? { outMs: 0 };
      if (dur > 0 && ms >= outMs) {
        setTimeMs(outMs);
        setPlaying(false);
      } else setTimeMs(ms);
    },
    [dur, trimRangeRef, setTimeMs, setPlaying],
  );

  const aimAt = useCallback(
    (x: number, y: number) => {
      if (selRef.current)
        void applyOp({ op: "update_zoom", id: selRef.current, target: { fixed: { x, y } } });
    },
    [applyOp],
  );

  const onMoveMode = useCallback(
    (want: boolean) => {
      if (want) setAimOn(false);
      requestMoveMode(want);
    },
    [setAimOn, requestMoveMode],
  );

  const onSeek = useCallback(
    (ms: number) => {
      setPlaying(false);
      setTimeMs(ms);
    },
    [setPlaying, setTimeMs],
  );

  const onPlayToggle = useCallback(
    () =>
      setPlaying((p: boolean) => {
        const t = timeMsRef.current,
          range = trimRangeRef.current;
        if (!p && range && (t < range.inMs || t >= range.outMs)) setTimeMs(range.inMs);
        return !p;
      }),
    [setPlaying, setTimeMs, timeMsRef, trimRangeRef],
  );

  const onAspect = useCallback(
    (aspect: Aspect) => {
      void applyOp({ op: "set_aspect", aspect });
    },
    [applyOp],
  );
  const cycleQuality = useCallback(
    () => setQuality((q) => (q === 480 ? 720 : q === 720 ? 1080 : 480)),
    [setQuality],
  );
  const onMuteToggle = useCallback(() => setMuted((m) => !m), [setMuted]);

  return { onTime, aimAt, onMoveMode, onSeek, onPlayToggle, onAspect, cycleQuality, onMuteToggle };
}
