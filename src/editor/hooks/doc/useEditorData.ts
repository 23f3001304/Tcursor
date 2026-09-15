import { useEffect, useState } from "react";
import {
  getEdit,
  setCapturable,
  cameraTrack,
  previewLayout,
  previewLayouts,
  clickTrack,
} from "../../../shared/ipc";
import type { EditDoc } from "../../../shared/edit";
import type { CamSample, ClickSample, PreviewLayout, LayoutPresets } from "../../../shared/ipc";
import { useCursorData } from "./useCursorData";
import { useMediaAssets } from "./useMediaAssets";
import { usePreviewBg } from "./usePreviewBg";

export function useEditorData(folder: string, rev: number, quality: number) {
  const [doc, setDoc] = useState<EditDoc | null>(null);
  const [track, setTrack] = useState<CamSample[]>([]);
  const [layout, setLayout] = useState<PreviewLayout | null>(null);
  const [layoutPresets, setLayoutPresets] = useState<LayoutPresets | null>(null);
  const [clicks, setClicks] = useState<ClickSample[]>([]);
  const [playing, setPlaying] = useState(false);

  useEffect(() => {
    let live = true;
    getEdit(folder)
      .then((d) => {
        if (live) setDoc(d);
      })
      .catch(() => {});
    void setCapturable(true);
    return () => {
      live = false;
    };
  }, [folder]);
  useEffect(() => {
    let live = true;
    cameraTrack(folder)
      .then((d) => {
        if (live) setTrack(d);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [folder, rev]);
  useEffect(() => {
    let live = true;
    previewLayout(folder)
      .then((d) => {
        if (live) setLayout(d);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [folder, rev]);
  useEffect(() => {
    let live = true;
    previewLayouts(folder)
      .then((d) => {
        if (live) setLayoutPresets(d);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [folder, rev]);
  useEffect(() => {
    let live = true;
    clickTrack(folder)
      .then((d) => {
        if (live) setClicks(d);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [folder]);

  const bg = usePreviewBg(folder, doc);
  const cursor = useCursorData(folder, doc);
  const media = useMediaAssets(folder, quality, setPlaying);

  return {
    doc,
    setDoc,
    track,
    layout,
    layoutPresets,
    clicks,
    bg,
    ...cursor,
    ...media,
    playing,
    setPlaying,
  };
}
