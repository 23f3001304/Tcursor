import { useEffect, useMemo, useRef, type RefObject } from "react";
import { previewFrame } from "../../../shared/ipc";

export interface ExactFrame {
  key: string;
  img: HTMLImageElement;
}

export const SETTLE_MS = 160;

export function exactKey(outMs: number, editGen: number): string {
  return `${Math.max(0, Math.round(outMs))}|${editGen}`;
}

export function wantsExact(
  playing: boolean,
  draft: boolean,
  folder: string,
  key: string,
  held: ExactFrame | null,
): boolean {
  return !playing && !draft && folder !== "" && held?.key !== key;
}

export function useExactFrame({
  folder,
  playing,
  outMs,
  draft,
  dirtyRef,
  deps,
}: {
  folder: string;
  playing: boolean;
  outMs: number;
  draft: boolean;
  dirtyRef: RefObject<boolean>;
  deps: unknown[];
}): { exactRef: RefObject<ExactFrame | null>; editGenRef: RefObject<number> } {
  const genRef = useRef(0);
  const editGen = useMemo(() => ++genRef.current, deps); // eslint-disable-line react-hooks/exhaustive-deps
  const editGenRef = useRef(editGen);
  editGenRef.current = editGen;
  const exactRef = useRef<ExactFrame | null>(null);

  useEffect(() => {
    const key = exactKey(outMs, editGen);
    if (!wantsExact(playing, draft, folder, key, exactRef.current)) return;
    let live = true;
    const timer = setTimeout(() => {
      previewFrame(folder, Math.max(0, Math.round(outMs)))
        .then((url) => {
          if (!live) return;
          const img = new Image();
          img.onload = () => {
            if (!live) return;
            exactRef.current = { key, img };
            dirtyRef.current = true;
          };
          img.src = url;
        })
        .catch(() => {});
    }, SETTLE_MS);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [folder, playing, outMs, draft, editGen]); // eslint-disable-line react-hooks/exhaustive-deps

  return { exactRef, editGenRef };
}
