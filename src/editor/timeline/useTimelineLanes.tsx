import { useCallback, useMemo, type RefObject } from "react";
import { IconZoomIn, IconBulb } from "@tabler/icons-react";
import type { EditDoc, EditOp } from "../../shared/edit";
import type { LayoutPresets } from "../../shared/ipc";
import { useDensity } from "../shell/useDensity";
import { layoutRegions } from "./model/layers";
import { useLaneDrag } from "./useLaneDrag";
import type { BeginDrag, Drag } from "../hooks/input/useRegionDrag";
import { useLayoutLaneRegions, layoutLabel, layoutExtraStyle } from "./lanes/LayoutLane";
import { useCaptionLaneRegions, captionLabel, captionTitle } from "./lanes/CaptionLane";
import { AudioTrack } from "./lanes/AudioTrack";
import { CameraLane } from "./lanes/CameraLane";
import { TimeLane } from "./lanes/TimeLane";
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

const laneHeight = (rows: number, rowH: number, gap: number) =>
  rows > 0 ? rows * rowH + (rows - 1) * gap : 0;

const zoomLabel = (z: { scale: number }) => (
  <>
    <IconZoomIn size={12} />
    {z.scale.toFixed(1)}x
  </>
);
const fxLabel = () => (
  <>
    <IconBulb size={12} />
    Spotlight
  </>
);

export function useTimelineLanes({
  doc,
  dur,
  sel,
  onSel,
  onApply,
  track,
  waves,
  wavesReady,
  hasWebcam,
  layoutPresets,
}: {
  doc: EditDoc;
  dur: number;
  sel: string | null;
  onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  track: RefObject<HTMLDivElement | null>;
  waves: { system: string; mic: string };
  wavesReady: boolean;
  hasWebcam: boolean;
  layoutPresets: LayoutPresets | null;
}): TimelineLane[] {
  const { row: ROW_H, audioRow: AUDIO_ROW_H, gap: GAP } = useDensity();
  const zooms = useMemo(() => layoutRegions(doc.zooms), [doc.zooms]);
  const fx = useMemo(() => layoutRegions(doc.effects), [doc.effects]);
  const layouts = useLayoutLaneRegions(doc.layout, layoutPresets);
  const captions = useCaptionLaneRegions(doc.captions);

  const onCommitZoom = useCallback(
    (id: string, start_ms: number, end_ms: number, layer: number) =>
      void onApply({ op: "update_zoom", id, start_ms, end_ms, layer }),
    [onApply],
  );
  const onCommitFx = useCallback(
    (id: string, start_ms: number, end_ms: number, layer: number) =>
      void onApply({ op: "update_effect", id, start_ms, end_ms, layer }),
    [onApply],
  );
  const onCommitLayout = useCallback(
    (id: string, start_ms: number, end_ms: number) =>
      void onApply({ op: "update_layout_seg", id, start_ms, end_ms }),
    [onApply],
  );
  const onCommitCaption = useCallback(
    (id: string, start_ms: number, end_ms: number) =>
      void onApply({ op: "update_caption", id, start_ms, end_ms }),
    [onApply],
  );

  const zoom = useLaneDrag(zooms, dur, track, ROW_H, onCommitZoom, onSel);
  const eff = useLaneDrag(fx, dur, track, ROW_H, onCommitFx, onSel);
  const lay = useLaneDrag(layouts, dur, track, ROW_H, onCommitLayout, onSel);
  const cap = useLaneDrag(captions, dur, track, ROW_H, onCommitCaption, onSel);

  const audioRows = !wavesReady ? 2 : (waves.system ? 1 : 0) + (waves.mic ? 1 : 0);
  const isSel = (regions: { id: string }[]) => sel != null && regions.some((r) => r.id === sel);
  const lanes: TimelineLane[] = [];
  if (doc.cuts.length || doc.speed.length)
    lanes.push({
      key: "time",
      label: "Time",
      heightPx: ROW_H,
      active: isSel(doc.speed),
      body: <TimeLane doc={doc} dur={dur} sel={sel} onSel={onSel} onApply={onApply} track={track} />,
    });
  const regionLane = <T extends Region>(
    key: string,
    label: string,
    lane: { rows: number; drag: Drag | null; beginDrag: BeginDrag },
    regions: T[],
    rowClass: string,
    blkClass: string,
    renderLabel: (r: T) => React.ReactNode,
    extraStyle?: (r: T, s: number, e: number) => React.CSSProperties,
    titleOf?: (r: T) => string | undefined,
  ) => {
    if (lane.rows === 0) return;
    lanes.push({
      key,
      label,
      heightPx: laneHeight(lane.rows, ROW_H, GAP),
      active: isSel(regions),
      body: (
        <RegionRows
          rows={lane.rows}
          regions={regions}
          dur={dur}
          sel={sel}
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
  };
  regionLane("zoom", "Zoom", zoom, zooms, "e-zoomrow", "e-zblk", zoomLabel);
  regionLane("fx", "FX", eff, fx, "e-fxrow", "e-fxblk", fxLabel);
  regionLane(
    "captions",
    "Captions",
    cap,
    captions,
    "e-caprow",
    "e-capblk",
    captionLabel,
    undefined,
    captionTitle,
  );
  regionLane("layout", "Layout", lay, layouts, "e-layrow", "e-layblk", layoutLabel, layoutExtraStyle);
  lanes.push({
    key: "camera",
    label: "Camera",
    heightPx: ROW_H,
    active: isSel(doc.camera_moves),
    body: (
      <CameraLane
        doc={doc}
        dur={dur}
        sel={sel}
        onSel={onSel}
        onApply={onApply}
        track={track}
        hasWebcam={hasWebcam}
      />
    ),
  });
  if (!wavesReady || waves.system || waves.mic)
    lanes.push({
      key: "audio",
      label: "Audio",
      heightPx: laneHeight(audioRows, AUDIO_ROW_H, GAP),
      active: false,
      body: (
        <>
          <AudioTrack src={waves.system} kind="system" loading={!wavesReady} />
          <AudioTrack src={waves.mic} kind="mic" loading={!wavesReady} />
        </>
      ),
    });
  return lanes;
}
