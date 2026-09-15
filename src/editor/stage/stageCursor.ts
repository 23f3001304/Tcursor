import type { CursorLayerDto } from "../../shared/ipc";
import type { CursorSettings } from "../../hud/settings/settings";

export function stageCursor(
  cursor: CursorSettings,
  osCursorInVideo: boolean,
  cursorLayer: CursorLayerDto | null,
) {
  const captured = cursor.style === "system" ? cursorLayer : null;
  const plainOs = cursor.style === "system" && !osCursorInVideo && !captured;
  const effCursor: CursorSettings = plainOs
    ? { ...cursor, style: "enhanced", click_bounce: false, motion_blur: 0, tilt: 0, back: "none" }
    : cursor;
  return { captured, plainOs, effCursor };
}
