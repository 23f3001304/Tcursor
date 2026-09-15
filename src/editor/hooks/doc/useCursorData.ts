import { useEffect, useState } from "react";
import { cursorKinds, cursorLayer, cursorSprites, osCursorInVideo } from "../../../shared/ipc";
import type { CursorKindSample, CursorLayerDto, CursorPackDto } from "../../../shared/ipc";
import type { EditDoc } from "../../../shared/edit";

export function useCursorData(folder: string, doc: EditDoc | null) {
  const [cursorSpr, setCursorSpr] = useState<CursorPackDto | null>(null);
  const [cursorKnd, setCursorKnd] = useState<CursorKindSample[]>([]);
  const [cursorLyr, setCursorLyr] = useState<CursorLayerDto | null>(null);
  const [osCursor, setOsCursor] = useState(true);

  useEffect(() => {
    let live = true;
    cursorKinds(folder)
      .then((d) => {
        if (live) setCursorKnd(d);
      })
      .catch(() => {});
    osCursorInVideo(folder)
      .then((v) => {
        if (live) setOsCursor(v);
      })
      .catch(() => {});
    cursorLayer(folder)
      .then((l) => {
        if (live) setCursorLyr(l);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [folder]);

  useEffect(() => {
    if (!doc) return;
    let live = true;
    cursorSprites(folder)
      .then((s) => {
        if (live) setCursorSpr(s);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [folder, doc?.settings.cursor.pack]);

  return { cursorSpr, cursorKnd, cursorLyr, osCursor };
}
