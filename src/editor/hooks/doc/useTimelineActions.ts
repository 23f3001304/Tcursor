import { useCallback, type RefObject } from "react";
import type { CameraMove, EditDoc, EditOp } from "../../../shared/edit";

export function pickAddedCameraMoveId(before: CameraMove[], after: CameraMove[]): string | null {
  const beforeIds = new Set(before.map((m) => m.id));
  return after.find((m) => !beforeIds.has(m.id))?.id ?? null;
}

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
    const d = await applyOp({
      op: "add_camera_move",
      t_ms: Math.round(timeMsRef.current),
      x: 0.5,
      y: 0.5,
      size: 0.25,
    });
    const id = d && pickAddedCameraMoveId(before, d.camera_moves);
    if (id) setSel(id);
  }, [applyOp, timeMsRef, docRef, setSel]);
  const zoomAt = useCallback(
    async (x: number, y: number) => {
      setPlaying(false);
      const d = await applyOp({
        op: "add_zoom_full",
        at_ms: Math.round(timeMsRef.current),
        dur_ms: 2000,
        scale: 2.5,
      });
      if (d && d.zooms.length) {
        const id = d.zooms[d.zooms.length - 1].id;
        await applyOp({ op: "update_zoom", id, target: { fixed: { x, y } } });
        setSel(id);
      }
    },
    [applyOp, timeMsRef, setSel, setPlaying],
  );
  return { addZoom, addSpotlight, addCameraMove, zoomAt };
}
