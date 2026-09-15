import { useMemo } from "react";
import type { Caption } from "../../../shared/edit";
import { layoutRegions } from "../model/layers";

export type CaptionRegion = Caption & { layer: number };

export const PILL_CHARS = 22;

export const CAPTION_LABEL_MIN_PX = 56;

const captionRows = (captions: Caption[]): CaptionRegion[] => layoutRegions(captions);

export function useCaptionLaneRegions(captions: Caption[]): CaptionRegion[] {
  return useMemo(() => captionRows(captions), [captions]);
}

useCaptionLaneRegions.pure = captionRows;

export function captionPreviewText(text: string, max: number): string {
  const t = text.trim();
  if (t.length <= max) return t;
  const cut = t.lastIndexOf(" ", max);
  return (cut > 0 ? t.slice(0, cut) : t.slice(0, max)).trimEnd();
}

export const captionLabel = (c: CaptionRegion) => captionPreviewText(c.text, PILL_CHARS);

export const captionTitle = (c: CaptionRegion) => c.text.trim() || undefined;
