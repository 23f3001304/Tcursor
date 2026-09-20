// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { useSyncRefs } from "./useSyncRefs";
import type {
  CamSample,
  ClickSample,
  PreviewLayout,
  CursorKindSample,
  LayoutPresets,
} from "../../../shared/ipc";
import type {
  CaptionStyle,
  ClickFxSettings,
  CursorSettings,
  GradeSettings,
  ZoomSettings,
} from "../../../hud/settings/settings";
import type { CameraMove, Caption, EffectRegion, LayoutSeg, TextItem, Zoom } from "../../../shared/edit";
import type { TimeMap } from "../../../shared/math/remap";
import type { ClipDissolve } from "../../stage/clips/clipDissolve";

type Props = Parameters<typeof useSyncRefs>[0];
type Refs = ReturnType<typeof useSyncRefs>;

const mk = (n: number): Props => ({
  playing: n % 2 === 0,
  timeMs: n * 100,
  onTime: (_ms: number) => {},
  onSeek: (_ms: number) => {},
  track: [{ n }] as unknown as CamSample[],
  layout: { n } as unknown as PreviewLayout,
  clicks: [{ n }] as unknown as ClickSample[],
  effects: [{ n }] as unknown as EffectRegion[],
  clickfx: { n } as unknown as ClickFxSettings,
  grade: { n } as unknown as GradeSettings,
  captions: [{ n }] as unknown as Caption[],
  texts: [{ n }] as unknown as TextItem[],
  capStyle: { n } as unknown as CaptionStyle,
  accent: [n, n, n],
  cursorKinds: [{ n }] as unknown as CursorKindSample[],
  cursor: { n } as unknown as CursorSettings,
  layoutPresets: { n } as unknown as LayoutPresets,
  layoutSegs: [{ n }] as unknown as LayoutSeg[],
  cameraMoves: [{ n }] as unknown as CameraMove[],
  zooms: [{ n }] as unknown as Zoom[],
  zoomSettings: { n } as unknown as ZoomSettings,
  arranging: n % 2 === 1,
  map: { n } as unknown as TimeMap,
  dissolves: [{ n }] as unknown as ClipDissolve[],
  motionEasing: `smooth${n}`,
});

const PAIRS: [keyof Props, keyof Refs][] = [
  ["playing", "playRef"],
  ["timeMs", "timeRef"],
  ["onTime", "onTimeRef"],
  ["onSeek", "onSeekRef"],
  ["track", "trackRef"],
  ["layout", "layoutRef"],
  ["clicks", "clicksRef"],
  ["effects", "effectsRef"],
  ["clickfx", "clickfxRef"],
  ["grade", "gradeRef"],
  ["captions", "captionsRef"],
  ["texts", "textsRef"],
  ["capStyle", "capStyleRef"],
  ["accent", "accentRef"],
  ["cursorKinds", "kindsRef"],
  ["cursor", "cursorRef"],
  ["layoutPresets", "layoutPresetsRef"],
  ["layoutSegs", "layoutSegsRef"],
  ["cameraMoves", "cameraMovesRef"],
  ["zooms", "zoomsRef"],
  ["zoomSettings", "zoomSettingsRef"],
  ["arranging", "arrangingRef"],
  ["map", "mapRef"],
  ["dissolves", "dissolvesRef"],
  ["motionEasing", "motionEasingRef"],
];

let api: Refs;
let current: Props = mk(1);
let root: Root, container: HTMLDivElement;

function Harness() {
  api = useSyncRefs(current);
  return null;
}

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  current = mk(1);
  container = document.createElement("div");
  root = createRoot(container);
  act(() => {
    root.render(createElement(Harness));
  });
});
afterEach(() => {
  act(() => {
    root.unmount();
  });
});

describe("useSyncRefs", () => {
  it("holds every value passed on the first render", () => {
    for (const [propKey, refKey] of PAIRS) expect(api[refKey].current).toBe(current[propKey]);
  });

  it("holds the new value in every ref after a rerender", () => {
    current = mk(2);
    act(() => {
      root.render(createElement(Harness));
    });
    for (const [propKey, refKey] of PAIRS) expect(api[refKey].current).toBe(current[propKey]);
  });

  it("keeps every ref object's identity stable across a rerender", () => {
    const before = api;
    current = mk(2);
    act(() => {
      root.render(createElement(Harness));
    });
    for (const [, refKey] of PAIRS) expect(api[refKey]).toBe(before[refKey]);
  });
});
