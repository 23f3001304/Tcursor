import { useEffect, useRef } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { METER_W } from "../components/RecMeter";
import { keepHidden, morphWindow } from "../components/morph";

export const WIDTH = 360;

const CARD_PAD = 12 + 14;
const HEAD_H = 36;
const PREVIEW_H = 160;
const ROW_H = 46;
const TOG_H = 42;
const REC_H = 48;
const CARD_GAPS = 8 * 6 + 10;

export const CARD_HEIGHT = CARD_PAD + HEAD_H + PREVIEW_H + ROW_H * 3 + TOG_H + REC_H + CARD_GAPS + 2;

export const IDLE_HEIGHT = 16 + CARD_HEIGHT + 10;

export const MENU_OVERFLOW = 140;

const TAKE_PAD = 8 * 2;
const TAKE_GAP = 10;
const CAM_ROUND_W = 44;
const CLOCK_W = 6 + 9 + 6 + 52;
const SLOT_W = 4 + METER_W;
const TAKE_BTN_W = 42;
const TAKE_SAFETY = 10;

export interface PillParts {
  cam: boolean;
  meter: boolean;
  warn: boolean;
  sources: boolean;
}
const WARN_W = 20;

export const FULL_PILL: PillParts = { cam: true, meter: true, warn: false, sources: false };

export function takeWidth(parts: PillParts): number {
  return (
    TAKE_PAD +
    CLOCK_W +
    TAKE_BTN_W * 3 +
    TAKE_GAP * 3 +
    TAKE_SAFETY +
    (parts.cam ? CAM_ROUND_W + TAKE_GAP : 0) +
    (parts.meter ? SLOT_W + TAKE_GAP : 0) +
    (parts.warn ? WARN_W + TAKE_GAP : 0)
  );
}

export const TAKE_WIDTH = takeWidth(FULL_PILL);

export const TAKE_HEIGHT = 16 + 62 + 10;

const SRC_ROWS = 46 * 3,
  SRC_GAPS = 8 * 2,
  SRC_PAD = 4 + 10;

export const SOURCES_HEIGHT = SRC_ROWS + SRC_GAPS + SRC_PAD;

export const SOURCES_MENU_OVERFLOW = 262;

export type HudMode = "idle" | "recording" | "saving";

export function hudLogicalSize(
  menu: string | null,
  mode: HudMode,
  pill: PillParts = FULL_PILL,
): { width: number; height: number } {
  if (mode !== "idle") {
    const sheet = pill.sources ? SOURCES_HEIGHT + (menu ? SOURCES_MENU_OVERFLOW : 0) : 0;
    return { width: takeWidth(pill), height: TAKE_HEIGHT + sheet };
  }
  return { width: WIDTH, height: menu ? IDLE_HEIGHT + MENU_OVERFLOW : IDLE_HEIGHT };
}

export function useHudWindowSize(menu: string | null, mode: HudMode, pill: PillParts = FULL_PILL) {
  const win = getCurrentWindow();
  const centeredOnce = useRef(false);
  const latest = useRef({ menu, mode, pill });
  latest.current = { menu, mode, pill };

  const sizedMode = useRef(mode);
  const sizedSheet = useRef(pill.sources);

  const applySize = async (center: boolean) => {
    const { width, height } = hudLogicalSize(latest.current.menu, latest.current.mode, latest.current.pill);
    const was = { ...latest.current.pill, sources: sizedSheet.current };
    const from = hudLogicalSize(latest.current.menu, sizedMode.current, was);
    const glide =
      !center &&
      (latest.current.mode !== sizedMode.current || latest.current.pill.sources !== sizedSheet.current);
    sizedMode.current = latest.current.mode;
    sizedSheet.current = latest.current.pill.sources;
    if (glide) {
      await morphWindow(from.width, from.height, width, height, 220, "centre");
      return;
    }
    await win.setSize(new LogicalSize(width, height));
    if (center) await win.center();
    await keepHidden();
  };

  useEffect(() => {
    void applySize(!centeredOnce.current);
    centeredOnce.current = true;
  }, [menu, mode, pill.cam, pill.meter, pill.warn, pill.sources]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void win
      .onScaleChanged(() => void applySize(false))
      .then((u) => {
        if (disposed) u();
        else unlisten = u;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
}
