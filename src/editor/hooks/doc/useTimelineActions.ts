import { useCallback, type RefObject } from "react";
import type { CameraMove, Clip, EditDoc, EditOp, MaskKind, TextKind } from "../../../shared/edit";

function addedSince<T extends { id: string }>(before: T[], after: T[]): T[] {
  const beforeIds = new Set(before.map((x) => x.id));
  return after.filter((x) => !beforeIds.has(x.id));
}

export function pickAddedCameraMoveId(before: CameraMove[], after: CameraMove[]): string | null {
  return addedSince(before, after)[0]?.id ?? null;
}

export function pickSplitClipId(before: Clip[], after: Clip[], atMs: number): string | null {
  const created = addedSince(before, after);
  if (created.length <= 1) return created[0]?.id ?? null;
  return (created.find((c) => c.src_in_ms === atMs) ?? created[created.length - 1]).id;
}

export async function runSplitAt(
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
  atMs: number,
  clipsBefore: Clip[],
  setSel: (id: string | null) => void,
): Promise<void> {
  const at_ms = Math.round(atMs);
  const d = await applyOp({ op: "split_at", at_ms });
  const id = d && pickSplitClipId(clipsBefore, d.clips, at_ms);
  if (id) setSel(id);
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
  const addMask = useCallback(
    async (kind: MaskKind) => {
      const t = Math.round(timeMsRef.current);
      const d = await applyOp({ op: "add_effect", kind, start_ms: t, end_ms: t + 3000 });
      if (d && d.effects.length) setSel(d.effects[d.effects.length - 1].id);
    },
    [applyOp, timeMsRef, setSel],
  );
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
  const addText = useCallback(
    async (kind: TextKind) => {
      const at_ms = Math.round(timeMsRef.current);
      const d = await applyOp({ op: "add_text", at_ms, dur_ms: 3000, kind });
      if (d && d.texts.length) setSel(d.texts[d.texts.length - 1].id);
    },
    [applyOp, timeMsRef, setSel],
  );
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
  const splitAt = useCallback(
    () => runSplitAt(applyOp, timeMsRef.current, docRef.current?.clips ?? [], setSel),
    [applyOp, timeMsRef, docRef, setSel],
  );
  return { addZoom, addSpotlight, addMask, addCameraMove, addText, zoomAt, splitAt };
}
