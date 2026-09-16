// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { BackgroundThumb } from "../../../shared/ipc";
import type { EditDoc } from "../../../shared/edit";

const THUMBS: BackgroundThumb[] = [
  { id: "amber", name: "Amber", kind: "mesh", group: "Ribbons", png_base64: "AAA" },
  { id: "paper", name: "Paper", kind: "mesh", group: "Ribbons", png_base64: "" },
  { id: "folds-citrus", name: "Citrus", kind: "mesh", group: "Folds", png_base64: "CCC" },
  { id: "scenic-dunes", name: "Dunes", kind: "mesh", group: "Scenic", png_base64: "DDD" },
];
vi.mock("../../../shared/ipc", () => ({
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
    background: {
      kind: "mesh",
      mesh: "paper",
      solid: [0, 0, 0],
      gradient_from: [1, 2, 3],
      gradient_to: [7, 8, 9],
      gradient_angle_deg: 0,
      blur: 0,
      dim: 0,
      asset: null,
    },
    appearance: { screen: { pad: 0.03125, screen_radius: 0.016 } },
    ui: { accent: [239, 68, 68] },
    grade: { preset: "none", exposure: 0, contrast: 1, vignette: 0 },
  },
} as unknown as EditDoc;

const docWith = (mesh: string): EditDoc =>
  ({ ...DOC, settings: { ...DOC.settings, background: { ...DOC.settings.background, mesh } } }) as EditDoc;

const docGraded = (grade: EditDoc["settings"]["grade"]): EditDoc =>
  ({ ...DOC, settings: { ...DOC.settings, grade } }) as EditDoc;

let root: Root, container: HTMLDivElement;
let saved: EditDoc["settings"][] = [];

const sections = () => Array.from(container.querySelectorAll<HTMLElement>(".e-cat"));
const headOf = (i: number) => sections()[i].querySelector<HTMLButtonElement>(".e-cat-btn")!;
const labels = () => sections().map((s) => s.querySelector(".e-cat-label")?.textContent);
const openFlags = () => sections().map((s) => s.querySelector(".e-cat-btn")!.getAttribute("aria-expanded"));
const tilesOf = (i: number) => Array.from(sections()[i].querySelectorAll<HTMLButtonElement>(".e-tile"));

const show = async (doc: EditDoc = DOC) => {
  await act(async () => {
    root.render(
      <BackgroundPanel folder="C:\\rec" doc={doc} onClose={() => {}} onSaveSettings={(s) => saved.push(s)} />,
    );
  });
};

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  saved = [];
  localStorage.clear();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => {
  act(() => {
    root.unmount();
  });
  container.remove();
  localStorage.clear();
});

describe("BackgroundPanel wallpaper sections", () => {
  it("renders one collapsible section per group, named and counted, with Your file last", async () => {
    await show();
    expect(labels()).toEqual(["Ribbons", "Folds", "Scenic", "Your file"]);
    expect(sections().map((s) => s.querySelector(".e-cat-count")?.textContent)).toEqual(["3", "1", "1", "0"]);
  });

  it("opens only the section holding the current wallpaper", async () => {
    await show();
    expect(openFlags()).toEqual(["true", "false", "false", "false"]);
    expect(tilesOf(0).map((b) => b.textContent)).toEqual(["Classic", "Amber", "Paper"]);
    expect(tilesOf(1)).toHaveLength(0);
  });

  it("follows the selection into a later section", async () => {
    await show(docWith("scenic-dunes"));
    expect(openFlags()).toEqual(["false", "false", "true", "false"]);
    expect(tilesOf(2).map((b) => b.getAttribute("aria-pressed"))).toEqual(["true"]);
  });

  it("names the chosen wallpaper in a CLOSED section's header, so nothing has to be opened to see it", async () => {
    await show(docWith("folds-citrus"));
    act(() => {
      headOf(1).click();
    });
    expect(sections()[1].querySelector(".e-cat-sel")?.textContent).toBe("Citrus");
    expect(sections()[0].querySelector(".e-cat-sel")).toBeNull();
  });

  it("marks exactly one tile selected, across every section", async () => {
    await show();
    act(() => {
      headOf(1).click();
    });
    act(() => {
      headOf(2).click();
    });
    expect(container.querySelectorAll('.e-tile[aria-pressed="true"]')).toHaveLength(1);
  });

  it("writes the same settings patch a tile click always did", async () => {
    await show();
    act(() => {
      headOf(1).click();
    });
    act(() => {
      tilesOf(1)[0].click();
    });
    expect(saved).toHaveLength(1);
    expect(saved[0]).toEqual({
      ...DOC.settings,
      background: { ...DOC.settings.background, kind: "mesh", mesh: "folds-citrus" },
    });
  });

  it("never lays a picker out as a sideways scroller", async () => {
    await show();
    expect(container.querySelectorAll(".e-tilerow, .e-tile-strip")).toHaveLength(0);
    expect(container.querySelectorAll(".e-tile-grid").length).toBeGreaterThan(0);
  });

  it("puts the tuning sections under one More row, closed until asked", async () => {
    await show();
    expect(container.querySelectorAll(".e-more-btn")).toHaveLength(1);
    expect(container.textContent).not.toContain("Corner Radius");
    act(() => {
      container.querySelector<HTMLButtonElement>(".e-more-btn")!.click();
    });
    expect(container.textContent).toContain("Corner Radius");
    expect(container.textContent).toContain("Background Blur");
  });
});

describe("BackgroundPanel colour grade", () => {
  const look = () => container.querySelector<HTMLButtonElement>('[aria-label="Look"]')!;
  const options = () => Array.from(document.body.querySelectorAll<HTMLButtonElement>('[role="option"]'));

  it("offers the nine looks and writes all four grade fields in one save", async () => {
    await show();
    act(() => {
      look().click();
    });
    expect(options().map((o) => o.textContent)).toEqual([
      "None",
      "Cinematic",
      "Noir",
      "Vintage",
      "Frost",
      "Golden",
      "Midnight",
      "Vivid",
      "Dreamy",
    ]);
    act(() => {
      options()[1].click();
    });
    expect(saved).toHaveLength(1);
    expect(saved[0].grade).toEqual({
      preset: "cinematic",
      exposure: 0,
      contrast: 1.12,
      vignette: 0.28,
    });
  });

  it("puts the grade back to none when the panel is reset", async () => {
    await show(docGraded({ preset: "noir", exposure: 0.05, contrast: 1.3, vignette: 0.4 }));
    act(() => {
      container.querySelector<HTMLButtonElement>('[aria-label="Reset to defaults"]')!.click();
    });
    expect(saved[0].grade).toEqual({ preset: "none", exposure: 0, contrast: 1, vignette: 0 });
  });
});
