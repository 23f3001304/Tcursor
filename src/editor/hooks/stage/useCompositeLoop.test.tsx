// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { buildTimeMap, clipOf, outOf, type TimeMap } from "../../../shared/math/remap";
import type { CompositeLoopRefs } from "./compositeLoopRefs";
import { useCompositeLoop } from "./useCompositeLoop";

const clip = (id: string, a: number, b: number) => ({
  id,
  src_in_ms: a,
  src_out_ms: b,
  transition_in_ms: 0,
});

const mapOf = (clips: ReturnType<typeof clip>[]): TimeMap =>
  buildTimeMap({ in_ms: 0, out_ms: 0 }, [], [], clips, 6000);

const REORDERED = mapOf([clip("cl0", 3000, 6000), clip("cl1", 0, 3000)]);
const IN_ORDER = mapOf([clip("cl0", 0, 3000), clip("cl1", 3000, 6000)]);

const fakeMedia = () => ({ currentTime: 1, playbackRate: 1 }) as HTMLVideoElement;

function loopRefs(map: TimeMap, media: HTMLVideoElement[]) {
  return {
    screenRef: { current: media[0] },
    webcamRef: { current: media[1] },
    audioRef: { current: media[2] },
    canvasRef: { current: null },
    playRef: { current: false },
    timeRef: { current: 1000 },
    onTimeRef: { current: () => {} },
    onSeekRef: { current: () => {} },
    trailRef: { current: [] },
    dirtyRef: { current: false },
    spotSimRef: { current: null },
    mapRef: { current: map },
  } as unknown as CompositeLoopRefs;
}

let root: Root;
let container: HTMLDivElement;
let frame: (() => void) | null = null;

const mount = (refs: CompositeLoopRefs) => {
  const Harness = () => {
    useCompositeLoop(refs);
    return null;
  };
  act(() => {
    root.render(<Harness />);
  });
};

const tick = () =>
  act(() => {
    const f = frame;
    frame = null;
    f?.();
  });

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  frame = null;
  vi.stubGlobal("requestAnimationFrame", (cb: () => void) => {
    frame = cb;
    return 1;
  });
  vi.stubGlobal("cancelAnimationFrame", () => {
    frame = null;
  });
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
  vi.unstubAllGlobals();
});

describe("useCompositeLoop's output clock", () => {
  it("re-derives the clock from the element when an edit rebuilds the map under a playing take", () => {
    const media = [fakeMedia(), fakeMedia(), fakeMedia()];
    const refs = loopRefs(REORDERED, media);
    mount(refs);
    tick();
    refs.playRef.current = true;
    tick();
    expect(media.map((m) => m.currentTime)).toEqual([1, 1, 1]);
    expect(outOf(REORDERED, 1000)).toBe(4000);
    expect(outOf(IN_ORDER, 1000)).toBe(1000);
    expect(clipOf(IN_ORDER, 4000)).toBe(4000);
    refs.mapRef.current = IN_ORDER;
    tick();
    expect(media.map((m) => m.currentTime)).toEqual([1, 1, 1]);
  });
});
