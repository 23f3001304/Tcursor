// Reading an imported background asset for the panel: its name, its kind, the line under its
// name, and where its thumbnail lives. Pure, so the card itself stays a thin render.
//
// Two of these mirror Rust (`settings::bg_asset`): the extension table and the thumbnail path.
// They are duplicated rather than fetched because the card must be able to draw a tile before any
// IPC resolves - but both are one-liners with their Rust twin named in the comment, and both are
// pinned by tests here and there.
import type { BackgroundAssetInfo } from "../../lib/ipc";

const EXTS: Record<string, "image" | "video"> = {
  png: "image", jpg: "image", jpeg: "image", webp: "image",
  gif: "video", mp4: "video", webm: "video", mov: "video",
};

/** The file name out of a project-relative asset path. */
export function assetFileName(rel: string): string {
  return rel.slice(rel.lastIndexOf("/") + 1);
}

/** Which `BackgroundKind` this asset is, by extension. Mirrors `bg_asset::asset_kind_for`. */
export function assetKindOf(rel: string): "image" | "video" | null {
  const ext = rel.slice(rel.lastIndexOf(".") + 1).toLowerCase();
  return (rel.includes(".") && EXTS[ext]) || null;
}

/** The line under the file name. `null` is a resolved "the file is gone" (the card says so);
 *  `undefined` is "still asking", which says nothing rather than flashing a wrong answer. */
export function assetSubtitle(info: BackgroundAssetInfo | null | undefined): string {
  if (info === undefined) return "";
  if (info === null) return "File missing";
  const kind = info.kind === "video" ? "Video" : "Image";
  // A probe that failed reports zeros; print what is actually known instead of "0x0".
  const size = info.width > 0 && info.height > 0 ? `${info.width}x${info.height}` : "";
  const dur = info.duration_ms ? `${(info.duration_ms / 1000).toFixed(1)}s` : "";
  return [kind, [size, dur].filter(Boolean).join(", ")].filter(Boolean).join(" ");
}

/** Where the asset's thumbnail lives. Mirrors `bg_asset::thumb_rel`. */
export function thumbRel(rel: string): string {
  const i = rel.lastIndexOf("/");
  return i < 0 ? `.thumbs/${rel}.jpg` : `${rel.slice(0, i)}/.thumbs/${rel.slice(i + 1)}.jpg`;
}
