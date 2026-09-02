import { useCallback, useRef, type RefObject } from "react";
import type { Aspect, EditDoc, EditOp } from "../../lib/edit";

/** Stabilizes the fistful of `Editor`-level callbacks handed down as props to the memoized hot
 *  tree (`Stage`/`Transport`/`Timeline`/`EditorPanels`) - split out of `Editor.tsx` purely to stay
 *  under that file's line budget (it sits at the cap). `applyOp` itself stays in `Editor.tsx`
 *  (several OTHER hooks - `useMoveModeGuard`, `useTrimActions`, `useTimelineActions`, the AI
 *  director - all need it built first) and is passed in here already-stable; everything below is
 *  built from it with `useCallback`, and any per-tick value (`timeMs`, the derived `trimRange`) is
 *  read off a ref instead of closed over directly, so identity survives a pure playhead tick -
 *  without that, a `React.memo`'d child re-renders anyway because one of its props is "new" every
 *  tick even though nothing it actually draws from changed. See editor.md "render hygiene". */
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
  const { applyOp, dur, timeMsRef, trimRangeRef, setTimeMs, setPlaying, setAimOn, setQuality, setMuted, requestMoveMode } = args;
  // `sel` read fresh via a ref (mirrors `Editor.tsx`'s `docRef`) - `aimAt` only ever fires from a
  // user gesture (never a tick), but keeping it off the dep list avoids a spurious identity change
  // every time the selection changes for an unrelated reason.
  const selRef = useRef(args.sel); selRef.current = args.sel;

  // Clamp playback to the trim's out point (not just the clip end): at the trim-out, park the
  // playhead exactly on the boundary rather than the frame or two of overshoot the rAF loop
  // reports before the pause lands. `dur` is a plain param (not a ref) - it only changes when the
  // video's real duration loads or the trim is edited, never on a tick, so it's safe to depend on.
  const onTime = useCallback((ms: number) => {
    const { outMs } = trimRangeRef.current ?? { outMs: 0 };
    if (dur > 0 && ms >= outMs) { setTimeMs(outMs); setPlaying(false); }
    else setTimeMs(ms);
  }, [dur, trimRangeRef, setTimeMs, setPlaying]);

  const aimAt = useCallback((x: number, y: number) => {
    if (selRef.current) void applyOp({ op: "update_zoom", id: selRef.current, target: { fixed: { x, y } } });
  }, [applyOp]);

  const onMoveMode = useCallback((want: boolean) => { if (want) setAimOn(false); requestMoveMode(want); }, [setAimOn, requestMoveMode]);

  // Shared by both Transport (skip buttons aside) and Timeline (track-body scrub/drop) - the two
  // were identical inline arrows before this pass.
  const onSeek = useCallback((ms: number) => { setPlaying(false); setTimeMs(ms); }, [setPlaying, setTimeMs]);

  const onPlayToggle = useCallback(() => setPlaying((p: boolean) => {
    // Starting playback outside the trim range snaps forward to the trim-in point first, so play
    // never starts inside a dimmed (trimmed-out) region.
    const t = timeMsRef.current, range = trimRangeRef.current;
    if (!p && range && (t < range.inMs || t >= range.outMs)) setTimeMs(range.inMs);
    return !p;
  }), [setPlaying, setTimeMs, timeMsRef, trimRangeRef]);

  const onAspect = useCallback((aspect: Aspect) => { void applyOp({ op: "set_aspect", aspect }); }, [applyOp]);
  const cycleQuality = useCallback(() => setQuality((q) => (q === 480 ? 720 : q === 720 ? 1080 : 480)), [setQuality]);
  const onMuteToggle = useCallback(() => setMuted((m) => !m), [setMuted]);

  return { onTime, aimAt, onMoveMode, onSeek, onPlayToggle, onAspect, cycleQuality, onMuteToggle };
}
