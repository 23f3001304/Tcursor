import { useCallback, type RefObject } from "react";
import type { CameraMove, EditDoc, EditOp } from "../../lib/edit";

/** Which of `after`'s camera_moves is new relative to `before` - the just-added keyframe.
 *  Diffing IDS (not `[length-1]`) is required because `AddCameraMove` SORTS `camera_moves` by
 *  `t_ms` server-side (`api.rs`), unlike `add_zoom`/`add_effect`/`add_layout_seg`, which stay
 *  append-only - so the newest entry's INDEX moves the instant it lands anywhere but the end
 *  (M2). `null` when nothing new is found (e.g. the apply failed and `after` === `before`). */
export function pickAddedCameraMoveId(before: CameraMove[], after: CameraMove[]): string | null {
  const beforeIds = new Set(before.map((m) => m.id));
  return after.find((m) => !beforeIds.has(m.id))?.id ?? null;
}

// The "add a region at the playhead" handlers shared by the Rail/Transport quick-add buttons and
// the canvas double-click (zoomAt): each applies the op then selects the newly created region so
// the inspector opens on it immediately - add + focus, one gesture. Kept out of Editor so it stays
// under the line limit (mirrors useTrimActions.ts).
//
// `timeMsRef` (not a plain `timeMs: number`) + `useCallback` (render hygiene pass): the playhead
// changes every tick, but `applyOp`/`setSel`/`setPlaying` don't - reading the current time off a
// ref instead of closing over it directly means these four callbacks stay referentially stable
// across ticks, which `Stage`/`Transport`/`EditorPanels` (all `React.memo`'d) need to actually
// skip re-rendering for them.
export function useTimelineActions(
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
  timeMsRef: RefObject<number>,
  docRef: RefObject<EditDoc | null>,
  setSel: (id: string | null) => void,
  setPlaying: (p: boolean) => void,
) {
  const addZoom = useCallback(async () => {
    const d = await applyOp({ op: "add_zoom", at_ms: Math.round(timeMsRef.current), dur_ms: 2000 });
    if (d && d.zooms.length) setSel(d.zooms[d.zooms.length - 1].id);
  }, [applyOp, timeMsRef, setSel]);
  const addSpotlight = useCallback(async () => {
    const t = Math.round(timeMsRef.current);
    const d = await applyOp({ op: "add_effect", kind: "spotlight", start_ms: t, end_ms: t + 2000 });
    if (d && d.effects.length) setSel(d.effects[d.effects.length - 1].id);
  }, [applyOp, timeMsRef, setSel]);
  const addCameraMove = useCallback(async () => {
    const before = docRef.current?.camera_moves ?? [];
    const d = await applyOp({ op: "add_camera_move", t_ms: Math.round(timeMsRef.current), x: 0.5, y: 0.5, size: 0.25 });
    const id = d && pickAddedCameraMoveId(before, d.camera_moves);
    if (id) setSel(id);
  }, [applyOp, timeMsRef, docRef, setSel]);
  // Zoom-in-and-target from a canvas double-click: adds a full zoom, then aims it at the clicked
  // point in one follow-up op - pausing first so the new zoom doesn't animate away under the click.
  const zoomAt = useCallback(async (x: number, y: number) => {
    setPlaying(false);
    const d = await applyOp({ op: "add_zoom_full", at_ms: Math.round(timeMsRef.current), dur_ms: 2000, scale: 2.5 });
    if (d && d.zooms.length) {
      const id = d.zooms[d.zooms.length - 1].id;
      await applyOp({ op: "update_zoom", id, target: { fixed: { x, y } } });
      setSel(id);
    }
  }, [applyOp, timeMsRef, setSel, setPlaying]);
  return { addZoom, addSpotlight, addCameraMove, zoomAt };
}
