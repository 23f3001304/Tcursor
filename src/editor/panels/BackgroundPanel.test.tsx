import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { BackgroundThumb } from "../../lib/ipc";
import type { EditDoc } from "../../lib/edit";

// The panel fetches its thumbnails over IPC and its asset card opens a Tauri file dialog; neither
// exists in jsdom, so both seams are stubbed. Everything else - the rows, the keyboard, the
// settings patch - is the real component.
const THUMBS: BackgroundThumb[] = [
  { id: "amber", name: "Amber", kind: "mesh", group: "Ribbons", png_base64: "AAA" },
  { id: "paper", name: "Paper", kind: "mesh", group: "Ribbons", png_base64: "" },
  { id: "folds-citrus", name: "Citrus", kind: "mesh", group: "Folds", png_base64: "CCC" },
  { id: "scenic-dunes", name: "Dunes", kind: "mesh", group: "Scenic", png_base64: "DDD" },
];
vi.mock("../../lib/ipc", () => ({
  backgroundThumbs: () => Promise.resolve(THUMBS),
  backgroundAssetInfo: () => Promise.resolve(null),
  importBackgroundAsset: () => Promise.resolve(null),
  removeBackgroundAsset: () => Promise.resolve(),
  fileSrc: (p: string) => p,
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: () => Promise.resolve(null) }));

const { BackgroundPanel } = await import("./BackgroundPanel");

const DOC = {
  settings: {
    background: { kind: "mesh", mesh: "paper", solid: [0, 0, 0], gradient_from: [1, 2, 3],
      gradient_to: [7, 8, 9], gradient_angle_deg: 0, blur: 0, dim: 0, asset: null },
    appearance: { screen: { pad: 0.03125, screen_radius: 0.016 } },
    ui: { accent: [239, 68, 68] },
  },
} as unknown as EditDoc;

let root: Root, container: HTMLDivElement;
let saved: EditDoc["settings"][] = [];
let scrolled: string[] = [];

const rows = () => Array.from(container.querySelectorAll<HTMLElement>('[role="listbox"]'));
const tilesOfRow = (i: number) => Array.from(rows()[i].querySelectorAll<HTMLButtonElement>('[role="option"]'));

const show = async (doc: EditDoc = DOC) => {
  await act(async () => {
    root.render(<BackgroundPanel folder="C:\\rec" doc={doc} onClose={() => {}}
      onSaveSettings={(s) => saved.push(s)} />);
  });
};

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  saved = []; scrolled = [];
  // jsdom ships no scrollIntoView at all, so the row's own call needs a stand-in to observe.
  Element.prototype.scrollIntoView = function (this: Element) {
    scrolled.push(this.getAttribute("title") ?? "");
  } as Element["scrollIntoView"];
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => { act(() => { root.unmount(); }); container.remove(); localStorage.clear(); });

describe("BackgroundPanel wallpaper rows", () => {
  it("renders one listbox row per group, named by the group, with Classic leading the first", async () => {
    await show();
    expect(rows().map((r) => r.getAttribute("aria-label"))).toEqual(["Ribbons", "Folds", "Scenic"]);
    expect(tilesOfRow(0).map((b) => b.title)).toEqual(["Classic", "Amber", "Paper"]);
    expect(tilesOfRow(2).map((b) => b.title)).toEqual(["Dunes"]);
    // The group name is the ROW's own label now, not a section heading above a grid.
    expect(container.querySelector(".e-tilerow-label")?.textContent).toBe("Ribbons");
  });

  it("scrolls the selected tile into view once the thumbnails resolve", async () => {
    await show();
    expect(scrolled).toContain("Paper");
    expect(tilesOfRow(0)[2].getAttribute("aria-selected")).toBe("true");
    // Exactly one tile is ever selected, across every row.
    expect(container.querySelectorAll('[aria-selected="true"]')).toHaveLength(1);
  });

  it("moves the selection with the arrow keys, clamped at the ends of the row", async () => {
    await show();
    act(() => { tilesOfRow(0)[2].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true })); });
    expect(saved).toHaveLength(1);
    expect(saved[0].background.mesh).toBe("amber");
    // "Paper" is the last tile of Ribbons, so ArrowRight has nowhere to go and re-picks it.
    act(() => { tilesOfRow(0)[2].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })); });
    expect(saved[1].background.mesh).toBe("paper");
  });

  it("keeps only the selected tile in the tab order, so a row is one tab stop", async () => {
    await show();
    expect(tilesOfRow(0).map((b) => b.tabIndex)).toEqual([-1, -1, 0]);
    // A row holding no selection still offers its first tile, or the row would be unreachable.
    expect(tilesOfRow(1).map((b) => b.tabIndex)).toEqual([0]);
  });

  it("writes the same settings patch a tile click always did", async () => {
    await show();
    act(() => { tilesOfRow(1)[0].click(); });
    expect(saved).toHaveLength(1);
    expect(saved[0]).toEqual({
      ...DOC.settings,
      background: { ...DOC.settings.background, kind: "mesh", mesh: "folds-citrus" },
    });
  });

  it("puts the tuning sections under one More row, closed until asked", async () => {
    await show();
    expect(container.querySelectorAll(".e-more-btn")).toHaveLength(1);
    expect(container.textContent).not.toContain("Corner Radius");
    act(() => { container.querySelector<HTMLButtonElement>(".e-more-btn")!.click(); });
    expect(container.textContent).toContain("Corner Radius");
    expect(container.textContent).toContain("Background Blur");
    expect(container.textContent).toContain("Accent Colors");
  });
});
