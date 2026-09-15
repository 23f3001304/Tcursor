import type { BackgroundAssetInfo } from "../../../shared/ipc";

const EXTS: Record<string, "image" | "video"> = {
  png: "image",
  jpg: "image",
  jpeg: "image",
  webp: "image",
  gif: "video",
  mp4: "video",
  webm: "video",
  mov: "video",
};

export function assetFileName(rel: string): string {
  return rel.slice(rel.lastIndexOf("/") + 1);
}

export function assetKindOf(rel: string): "image" | "video" | null {
  const ext = rel.slice(rel.lastIndexOf(".") + 1).toLowerCase();
  return (rel.includes(".") && EXTS[ext]) || null;
}

export function assetSubtitle(info: BackgroundAssetInfo | null | undefined): string {
  if (info === undefined) return "";
  if (info === null) return "File missing";
  const kind = info.kind === "video" ? "Video" : "Image";
  const size = info.width > 0 && info.height > 0 ? `${info.width}x${info.height}` : "";
  const dur = info.duration_ms ? `${(info.duration_ms / 1000).toFixed(1)}s` : "";
  return [kind, [size, dur].filter(Boolean).join(", ")].filter(Boolean).join(" ");
}

export function thumbRel(rel: string): string {
  const i = rel.lastIndexOf("/");
  return i < 0 ? `.thumbs/${rel}.jpg` : `${rel.slice(0, i)}/.thumbs/${rel.slice(i + 1)}.jpg`;
}
