import { useMemo } from "react";
import { IconTypography } from "@tabler/icons-react";
import type { TextItem } from "../../../shared/edit";
import { layoutRegions } from "../model/layers";

export type TextRegion = TextItem & { layer: number };

export const TEXT_PILL_CHARS = 18;

const textRows = (texts: TextItem[]): TextRegion[] => layoutRegions(texts);

export function useTextLaneRegions(texts: TextItem[]): TextRegion[] {
  return useMemo(() => textRows(texts), [texts]);
}

useTextLaneRegions.pure = textRows;

export function textPreviewText(text: string, max: number): string {
  const t = text.trim();
  if (!t) return "Text";
  if (t.length <= max) return t;
  const cut = t.lastIndexOf(" ", max);
  return (cut > 0 ? t.slice(0, cut) : t.slice(0, max)).trimEnd();
}

export const textLabel = (t: TextRegion) => (
  <>
    <IconTypography size={12} />
    {textPreviewText(t.text, TEXT_PILL_CHARS)}
  </>
);

export const textTitle = (t: TextRegion) => t.text.trim() || undefined;
