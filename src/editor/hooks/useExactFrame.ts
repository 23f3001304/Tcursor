import { useEffect, useMemo, useRef, type RefObject } from "react";
import { previewFrame } from "../../lib/ipc";

/** One frame of the export at a paused instant, as the stage's overlay: the key it was made for
 *  and the decoded image. */
export interface ExactFrame { key: string; img: HTMLImageElement }

/** How long the playhead (or the doc) must hold still before the export's frame is asked for:
 *  long enough that a scrub does not fire a request per pixel, short enough that a pause feels
 *  answered. The live composite shows in the meantime. */
export const SETTLE_MS = 160;

/** The key an exact frame is filed under: the instant, and the edit generation it was rendered
 *  for. A frame from another instant or an older doc never matches, so it is never drawn stale. */
export function exactKey(timeMs: number, editGen: number): string {
  return `${Math.max(0, Math.round(timeMs))}|${editGen}`;
}

/** Whether this tick should ask for an exact frame: paused, a folder to ask about, no unsaved
 *  draft the file would not know about, and not already holding the frame for this key. */
export function wantsExact(playing: boolean, draft: boolean, folder: string, key: string, held: ExactFrame | null): boolean {
  return !playing && !draft && folder !== "" && held?.key !== key;
}

/** The stage's exact-frame path (owner ruling 2026-09-14: the export is the reference, the
 *  preview must match it). Whenever playback is paused or a scrub has settled, the composited
 *  frame at that instant is requested from the Rust renderer (`preview_frame`, the export's own
 *  compositor and effects) and the loop draws it over the live canvas, so what is looked at while
 *  editing IS a frame of the export; playing resumes the live composite. `deps` is everything
 *  that changes what a frame looks like except the playhead: bumping the edit generation retires
 *  the held frame (the file the renderer reads has changed) and a new one is requested after
 *  `SETTLE_MS`. While a Move or Arrange draft is live (`draft`) nothing is requested - the file
 *  does not know the draft, and a frame without it would contradict the drag. */
export function useExactFrame({ folder, playing, timeMs, draft, dirtyRef, deps }: {
  folder: string; playing: boolean; timeMs: number; draft: boolean; dirtyRef: RefObject<boolean>; deps: unknown[];
}): { exactRef: RefObject<ExactFrame | null>; editGenRef: RefObject<number> } {
  const genRef = useRef(0);
  // Computed during render, so the effect below re-runs when any draw-affecting prop changes.
  const editGen = useMemo(() => ++genRef.current, deps); // eslint-disable-line react-hooks/exhaustive-deps
  const editGenRef = useRef(editGen); editGenRef.current = editGen;
  const exactRef = useRef<ExactFrame | null>(null);

  useEffect(() => {
    const key = exactKey(timeMs, editGen);
    if (!wantsExact(playing, draft, folder, key, exactRef.current)) return;
    let live = true;
    const timer = setTimeout(() => {
      previewFrame(folder, Math.max(0, Math.round(timeMs))).then((url) => {
        if (!live) return;
        const img = new Image();
        img.onload = () => { if (!live) return; exactRef.current = { key, img }; dirtyRef.current = true; };
        img.src = url;
      }).catch(() => { /* the live composite stays; the next settle asks again */ });
    }, SETTLE_MS);
    // A newer instant, a newer doc, a play or a draft supersedes this request: its answer is dropped.
    return () => { live = false; clearTimeout(timer); };
  }, [folder, playing, timeMs, draft, editGen]); // eslint-disable-line react-hooks/exhaustive-deps

  return { exactRef, editGenRef };
}
