import { useEffect, useRef, useState } from "react";
import { fileSrc, previewBg } from "../../../shared/ipc";
import type { EditDoc } from "../../../shared/edit";
import { debounce } from "../../util/debounce";
import { bgAssetUrl, type StageBg } from "../../stage/canvas/stageBg";

const PREVIEW_BG_DEBOUNCE_MS = 80;

export function usePreviewBg(folder: string, doc: EditDoc | null): StageBg {
  const [bgUrl, setBgUrl] = useState("");
  const bgSeqRef = useRef(0);
  const bgLiveRef = useRef(true);
  const fetchBgRef = useRef<ReturnType<typeof debounce<[string]>> | null>(null);
  if (!fetchBgRef.current) {
    fetchBgRef.current = debounce((f: string) => {
      const seq = ++bgSeqRef.current;
      previewBg(f)
        .then((u) => {
          if (bgLiveRef.current && bgSeqRef.current === seq) setBgUrl(u);
        })
        .catch(() => {});
    }, PREVIEW_BG_DEBOUNCE_MS);
  }
  useEffect(() => {
    fetchBgRef.current!(folder);
  }, [folder, JSON.stringify(doc?.settings.background)]);
  useEffect(() => {
    bgLiveRef.current = true;
    return () => {
      bgLiveRef.current = false;
      fetchBgRef.current?.cancel();
    };
  }, []);

  const bgSet = doc?.settings.background;
  return {
    url: bgUrl,
    assetUrl: bgAssetUrl(folder, bgSet?.asset, bgSet?.kind ?? "mesh", fileSrc),
    assetPath: bgSet?.asset ?? "",
    kind: bgSet?.kind ?? "mesh",
    dim: bgSet?.dim ?? 0,
  };
}
