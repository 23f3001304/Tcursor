import { useCallback, useEffect, useRef, useState } from "react";
import type { EditDoc, LayoutSeg } from "../../lib/edit";
import { arrangeSeekMs, nextArrangeMode, NO_ARRANGE, type ArrangeMode } from "./arrangeMode";

/** Owns stage arrange mode for the editor: which layout segment's panel frames the stage is
 *  showing, and the ways in and out. The decision itself is `nextArrangeMode` (pure, unit tested);
 *  this hook supplies the events, derives the segment, and seeks into it on entry.
 *
 *  Escape is a LOCAL listener, live only while the mode is on, rather than a new action in
 *  `keymap.ts` - the same shape `DirectorScrim`/`CameraLane`/the dialogs already use for their own
 *  dismiss key, and it keeps a global Escape from acquiring a meaning every other surface would
 *  then have to opt out of. */
export function useArrangeMode(doc: EditDoc | null, sel: string | null, io: {
  setSel: (id: string | null) => void; timeMsRef: { current: number }; onSeek: (ms: number) => void;
}): { arrangeSeg: LayoutSeg | null; arrangeOn: boolean; onArrange: () => void; onSel: (id: string | null) => void } {
  const ioRef = useRef(io); ioRef.current = io; // read at call time, so the callbacks stay stable
  const [mode, setMode] = useState<ArrangeMode>(NO_ARRANGE);
  const docRef = useRef(doc); docRef.current = doc;
  const isLayout = (id: string | null) => (id && docRef.current?.layout.some((l) => l.id === id) ? id : null);
  // The selection, but only when it is a LAYOUT segment - selecting a zoom/effect/keyframe reads
  // as "not arranging anything" and exits, exactly like a deselect.
  const selSegId = isLayout(sel);
  useEffect(() => { setMode((m) => nextArrangeMode(m, { kind: "select", segId: selSegId })); }, [selSegId]);

  const seg = mode.segId ? doc?.layout.find((l) => l.id === mode.segId) ?? null : null;
  // The remembered segment being deleted (or the doc reloading without it) exits outright, so the
  // inspector's re-enter button can never point at a segment that is no longer there.
  const gone = mode.segId !== null && seg === null;
  useEffect(() => { if (gone) setMode((m) => nextArrangeMode(m, { kind: "gone" })); }, [gone]);

  useEffect(() => {
    if (!mode.on) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") setMode((m) => nextArrangeMode(m, { kind: "escape" })); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [mode.on]);

  // Entering seeks into the segment when the playhead is outside it - see `arrangeSeekMs`. Keyed
  // on the ENTRY (mode.on + which segment), so scrubbing back out afterwards is left alone.
  const segRef = useRef(seg); segRef.current = seg;
  useEffect(() => {
    const s = segRef.current;
    if (!mode.on || !s) return;
    const ms = arrangeSeekMs(s, ioRef.current.timeMsRef.current);
    if (ms !== null) ioRef.current.onSeek(ms);
  }, [mode.on, mode.segId]); // eslint-disable-line react-hooks/exhaustive-deps

  const onArrange = useCallback(() => setMode((m) => nextArrangeMode(m, { kind: "arrange" })), []);
  // The Timeline's own selection handler. Clicking a pill is an ENTRY GESTURE, not just a value
  // change: after an Escape `sel` still holds that segment, so re-clicking it sets no new state
  // and the `[selSegId]` effect above never fires - the pill would be a dead control. Dispatching
  // the select event here makes the re-click re-enter; the effect still covers every OTHER way the
  // selection moves (rail tab, delete, an inspector's close button).
  const onSel = useCallback((id: string | null) => {
    ioRef.current.setSel(id);
    setMode((m) => nextArrangeMode(m, { kind: "select", segId: isLayout(id) }));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps
  return { arrangeSeg: mode.on ? seg : null, arrangeOn: mode.on, onArrange, onSel };
}
