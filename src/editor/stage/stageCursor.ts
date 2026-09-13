import type { CursorSettings } from "../../hud/settings/settings";
import type { CursorLayerDto } from "../../lib/ipc";

/** What the stage actually draws for the cursor setting. "System" means the captured OS-cursor
 *  layer when the recording has one; a recording that baked no cursor and captured no layer has
 *  nothing original to show, so it is redrawn as a plain Enhanced pointer (no bounce, no trail) and
 *  the cursor-kind track is dropped so the sprite never changes shape. */
export function stageCursor(cursor: CursorSettings, osCursorInVideo: boolean, cursorLayer: CursorLayerDto | null) {
  const captured = cursor.style === "system" ? cursorLayer : null;
  const plainOs = cursor.style === "system" && !osCursorInVideo && !captured;
  const effCursor: CursorSettings = plainOs ? { ...cursor, style: "enhanced", click_bounce: false, motion_blur: 0 } : cursor;
  return { captured, plainOs, effCursor };
}
