import { describe, expect, it } from "vitest";
import type { Caption } from "../../../shared/edit";
import {
  CAPTION_LABEL_MIN_PX,
  captionLabel,
  captionPreviewText,
  captionTitle,
  useCaptionLaneRegions,
} from "./CaptionLane";

const NODE_FS = "node:fs";
const fs = (await import(NODE_FS)) as { readFileSync(p: string, enc: "utf8"): string };
const node = globalThis as unknown as { process: { cwd(): string } };
const CSS = fs.readFileSync(
  `${node.process.cwd().replace(/\\/g, "/")}/src/editor/timeline/timeline.css`,
  "utf8",
);

const cap = (id: string, start_ms: number, end_ms: number, text: string): Caption => ({
  id,
  start_ms,
  end_ms,
  text,
  words: [],
});

describe("the caption lane", () => {
  it("keeps non-overlapping captions on one row", () => {
    const rows = useCaptionLaneRegions.pure([cap("c0", 0, 1000, "a"), cap("c1", 1000, 2000, "b")]);
    expect(rows.map((r) => r.layer)).toEqual([0, 0]);
  });

  it("stacks a caption that overlaps another so neither is hidden", () => {
    const rows = useCaptionLaneRegions.pure([cap("c0", 0, 2000, "a"), cap("c1", 1000, 3000, "b")]);
    expect(new Set(rows.map((r) => r.layer)).size).toBe(2);
  });

  it("shortens the pill label without cutting a word in half", () => {
    expect(captionPreviewText("hello there world", 11)).toBe("hello there");
    expect(captionPreviewText("short", 11)).toBe("short");
    expect(captionPreviewText("", 11)).toBe("");
  });

  it("says nothing at all rather than showing an ellipsis on an empty caption", () => {
    expect(captionPreviewText("   ", 11)).toBe("");
  });

  it("puts the caption text alone on the pill, with no icon to shear into a caret", () => {
    const label = captionLabel({ id: "c", start_ms: 0, end_ms: 1, text: "hello", words: [], layer: 0 });
    expect(label).toBe("hello");
  });

  it("carries the full line as the pill's own tooltip, and none at all when it is empty", () => {
    const region = { id: "c", start_ms: 0, end_ms: 1, words: [], layer: 0 };
    expect(captionTitle({ ...region, text: " a long spoken line " })).toBe("a long spoken line");
    expect(captionTitle({ ...region, text: "  " })).toBeUndefined();
  });

  it("hides the pill's label below the caption lane's own width rung, not the shared one", () => {
    expect(CSS).toContain(`@container (max-width: ${CAPTION_LABEL_MIN_PX - 0.1}px) { .e-capblk .e-zlabel`);
    expect(CSS.indexOf(".e-capblk .e-zlabel { display: block")).toBeGreaterThan(
      CSS.indexOf("@container (max-width: 46px)"),
    );
  });

  it("wears the audio hue, so the speech track reads with the waveforms", () => {
    expect(CSS).toContain(".e-capblk { --pill-accent: var(--e-wave); }");
    expect(CSS).toContain(".e-lane-captions { --lane-accent: var(--e-wave); }");
  });
});
