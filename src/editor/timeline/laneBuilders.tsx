import { IconZoomIn, IconBulb } from "@tabler/icons-react";
import type { BeginDrag, Drag } from "../hooks/input/useRegionDrag";
import { RegionRows } from "./lanes/RegionRows";

interface Region {
  id: string;
  start_ms: number;
  end_ms: number;
  layer: number;
}

export interface TimelineLane {
  key: string;
  label: string;
  heightPx: number;
  active: boolean;
  body: React.ReactNode;
}

export const laneHeight = (rows: number, rowH: number, gap: number) =>
  rows > 0 ? rows * rowH + (rows - 1) * gap : 0;

export const zoomLabel = (z: { scale: number }) => (
  <>
    <IconZoomIn size={12} />
    {z.scale.toFixed(1)}x
  </>
);
export const fxLabel = () => (
  <>
    <IconBulb size={12} />
    Spotlight
  </>
);

export interface LaneCtx {
  ROW_H: number;
  GAP: number;
  dur: number;
  sel: string | null;
  isSel: (regions: { id: string }[]) => boolean;
}

export function regionLane<T extends Region>(
  lanes: TimelineLane[],
  cx: LaneCtx,
  key: string,
  label: string,
  lane: { rows: number; drag: Drag | null; beginDrag: BeginDrag },
  regions: T[],
  rowClass: string,
  blkClass: string,
  renderLabel: (r: T) => React.ReactNode,
  extraStyle?: (r: T, s: number, e: number) => React.CSSProperties,
  titleOf?: (r: T) => string | undefined,
) {
  if (lane.rows === 0) return;
  lanes.push({
    key,
    label,
    heightPx: laneHeight(lane.rows, cx.ROW_H, cx.GAP),
    active: cx.isSel(regions),
    body: (
      <RegionRows
        rows={lane.rows}
        regions={regions}
        dur={cx.dur}
        sel={cx.sel}
        rowClass={rowClass}
        blkClass={blkClass}
        dragState={lane.drag}
        beginDrag={lane.beginDrag}
        renderLabel={renderLabel}
        extraStyle={extraStyle}
        titleOf={titleOf}
      />
    ),
  });
}
