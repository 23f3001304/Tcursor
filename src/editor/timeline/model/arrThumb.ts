import type { PanelRectDto } from "../../../shared/ipc";
import type { ResolvedPanels } from "./layoutTrack";
import { VISIBLE_ALPHA } from "../../stage/arrange/arrangeMath";

export const THUMB_W = 24,
  THUMB_H = 14;

export interface ThumbBox {
  x: number;
  y: number;
  w: number;
  h: number;
}
export interface ArrThumb {
  screen: ThumbBox | null;
  cam: ThumbBox | null;
}

function box(p: PanelRectDto, w: number, h: number): ThumbBox | null {
  return p.alpha > VISIBLE_ALPHA
    ? { x: p.rect[0] * w, y: p.rect[1] * h, w: p.rect[2] * w, h: p.rect[3] * h }
    : null;
}

export function arrThumb(panels: ResolvedPanels | null, w = THUMB_W, h = THUMB_H): ArrThumb {
  if (!panels) return { screen: null, cam: null };
  return { screen: box(panels.screen, w, h), cam: box(panels.cam, w, h) };
}
