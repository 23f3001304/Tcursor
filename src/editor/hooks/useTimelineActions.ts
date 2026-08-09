import type { EditDoc, EditOp } from "../../lib/edit";

// The "add a region at the playhead" handlers shared by the Rail/Transport quick-add buttons and
// the canvas double-click (zoomAt): each applies the op then selects the newly created region so
// the inspector opens on it immediately - add + focus, one gesture. Kept out of Editor so it stays
// under the line limit (mirrors useTrimActions.ts).
export function useTimelineActions(
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
  timeMs: number,
  setSel: (id: string | null) => void,
  setPlaying: (p: boolean) => void,
) {
  const addZoom = async () => {
    const d = await applyOp({ op: "add_zoom", at_ms: Math.round(timeMs), dur_ms: 2000 });
    if (d && d.zooms.length) setSel(d.zooms[d.zooms.length - 1].id);
  };
  const addSpotlight = async () => {
    const d = await applyOp({ op: "add_effect", kind: "spotlight", start_ms: Math.round(timeMs), end_ms: Math.round(timeMs) + 2000 });
    if (d && d.effects.length) setSel(d.effects[d.effects.length - 1].id);
  };
  const addCameraMove = async () => {
    const d = await applyOp({ op: "add_camera_move", t_ms: Math.round(timeMs), x: 0.5, y: 0.5, size: 0.25 });
    if (d && d.camera_moves.length) setSel(d.camera_moves[d.camera_moves.length - 1].id);
  };
  // Zoom-in-and-target from a canvas double-click: adds a full zoom, then aims it at the clicked
  // point in one follow-up op - pausing first so the new zoom doesn't animate away under the click.
  const zoomAt = async (x: number, y: number) => {
    setPlaying(false);
    const d = await applyOp({ op: "add_zoom_full", at_ms: Math.round(timeMs), dur_ms: 2000, scale: 2.5 });
    if (d && d.zooms.length) {
      const id = d.zooms[d.zooms.length - 1].id;
      await applyOp({ op: "update_zoom", id, target: { fixed: { x, y } } });
      setSel(id);
    }
  };
  return { addZoom, addSpotlight, addCameraMove, zoomAt };
}
