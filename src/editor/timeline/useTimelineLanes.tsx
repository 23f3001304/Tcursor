import { useCallback, useMemo, type RefObject } from "react";
import type { EditDoc, EditOp } from "../../shared/edit";
import type { LayoutPresets } from "../../shared/ipc";
import type { TimeMap } from "../../shared/math/remap";
import { useDensity } from "../shell/useDensity";
import { layoutRegions } from "./model/layers";
import { clipRegions } from "./model/clipModel";
import { useLaneDrag } from "./useLaneDrag";
import { useLayoutLaneRegions, layoutLabel, layoutExtraStyle } from "./lanes/LayoutLane";
import { useCaptionLaneRegions, captionLabel, captionTitle } from "./lanes/CaptionLane";
import { useTextLaneRegions, textLabel, textTitle } from "./lanes/TextLane";
import { AudioTrack } from "./lanes/AudioTrack";
import { CameraLane } from "./lanes/CameraLane";
import { ClipLane } from "./lanes/ClipLane";
import { TimeLane } from "./lanes/TimeLane";
import { laneHeight, zoomLabel, fxLabel, regionLane, type TimelineLane } from "./laneBuilders";

export type { TimelineLane } from "./laneBuilders";

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
  map,
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
  map: TimeMap;
}): TimelineLane[] {
  const { row: ROW_H, audioRow: AUDIO_ROW_H, gap: GAP } = useDensity();
  const clipRows = useMemo(
    () => (doc.clips.length > 1 ? Math.max(1, ...clipRegions(doc.clips, map).map((r) => r.layer + 1)) : 0),
    [doc.clips, map],
  );
  const zooms = useMemo(() => layoutRegions(doc.zooms), [doc.zooms]);
  const fx = useMemo(() => layoutRegions(doc.effects), [doc.effects]);
  const layouts = useLayoutLaneRegions(doc.layout, layoutPresets);
  const captions = useCaptionLaneRegions(doc.captions);
  const texts = useTextLaneRegions(doc.texts);

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

  const onCommitText = useCallback(
    (id: string, start_ms: number, end_ms: number) =>
      void onApply({ op: "update_text", id, start_ms, end_ms }),
    [onApply],
  );

  const zoom = useLaneDrag(zooms, dur, track, ROW_H, onCommitZoom, onSel);
  const eff = useLaneDrag(fx, dur, track, ROW_H, onCommitFx, onSel);
  const lay = useLaneDrag(layouts, dur, track, ROW_H, onCommitLayout, onSel);
  const cap = useLaneDrag(captions, dur, track, ROW_H, onCommitCaption, onSel);
  const txt = useLaneDrag(texts, dur, track, ROW_H, onCommitText, onSel);

  const audioRows = !wavesReady ? 2 : (waves.system ? 1 : 0) + (waves.mic ? 1 : 0);
  const isSel = (regions: { id: string }[]) => sel != null && regions.some((r) => r.id === sel);
  const cx = { ROW_H, GAP, dur, sel, isSel };
  const lanes: TimelineLane[] = [];
  if (clipRows > 0)
    lanes.push({
      key: "clips",
      label: "Clips",
      heightPx: laneHeight(clipRows, ROW_H, GAP),
      active: isSel(doc.clips),
      body: (
        <ClipLane
          clips={doc.clips}
          map={map}
          dur={dur}
          sel={sel}
          onSel={onSel}
          onApply={onApply}
          track={track}
        />
      ),
    });
  if (doc.cuts.length || doc.speed.length)
    lanes.push({
      key: "time",
      label: "Time",
      heightPx: ROW_H,
      active: isSel(doc.speed),
      body: <TimeLane doc={doc} dur={dur} sel={sel} onSel={onSel} onApply={onApply} track={track} />,
    });
  regionLane(lanes, cx, "zoom", "Zoom", zoom, zooms, "e-zoomrow", "e-zblk", zoomLabel);
  regionLane(lanes, cx, "fx", "FX", eff, fx, "e-fxrow", "e-fxblk", fxLabel);
  regionLane(
    lanes,
    cx,
    "text",
    "Text",
    txt,
    texts,
    "e-textrow",
    "e-textblk",
    textLabel,
    undefined,
    textTitle,
  );
  regionLane(
    lanes,
    cx,
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
  regionLane(
    lanes,
    cx,
    "layout",
    "Layout",
    lay,
    layouts,
    "e-layrow",
    "e-layblk",
    layoutLabel,
    layoutExtraStyle,
  );
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
