import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { useArrangeMode } from "./useArrangeMode";
import type { EditDoc, LayoutSeg } from "../../lib/edit";

// The hook-level counterpart to `arrangeMode.test.ts`: those pin the pure decision, these pin the
// WIRING - specifically that every entry gesture the brief names actually reaches it. Uses React's
// own `act` + `createRoot` against vitest's jsdom environment (no new dependency).
const seg = (id: string, over: Partial<LayoutSeg> = {}): LayoutSeg => ({
  id, start_ms: 1000, end_ms: 3000, layout: "camera", transition_ms: 200, easing: "smooth",
  transition_out_ms: 0, easing_out: "smooth", ...over,
});
const DOC = { layout: [seg("s1"), seg("s2", { start_ms: 4000, end_ms: 5000 })], zooms: [] } as unknown as EditDoc;

let api: ReturnType<typeof useArrangeMode>;
const timeMsRef = { current: 0 };
let seeks: number[] = [];
let root: Root, container: HTMLDivElement;

function Harness() {
  const [sel, setSel] = useState<string | null>(null);
  api = useArrangeMode(DOC, sel, { setSel, timeMsRef, onSeek: (ms) => seeks.push(ms) });
  return null;
}
const esc = () => act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })); });
const click = (id: string | null) => act(() => { api.onSel(id); });

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  timeMsRef.current = 0; seeks = [];
  container = document.createElement("div");
  root = createRoot(container);
  act(() => { root.render(<Harness />); });
});
afterEach(() => { act(() => { root.unmount(); }); });

describe("useArrangeMode", () => {
  it("starts off, and a pill click enters for that segment", () => {
    expect(api.arrangeOn).toBe(false);
    click("s1");
    expect(api.arrangeOn).toBe(true);
    expect(api.arrangeSeg?.id).toBe("s1");
  });

  it("RE-CLICKING the same pill after Escape re-enters, even though `sel` never changed", () => {
    click("s1");
    esc();
    expect(api.arrangeOn).toBe(false);
    click("s1"); // the regression: a value-change-keyed effect sees nothing here
    expect(api.arrangeOn).toBe(true);
    expect(api.arrangeSeg?.id).toBe("s1");
  });

  it("the inspector's button also re-enters after Escape, on the remembered segment", () => {
    click("s1");
    esc();
    act(() => { api.onArrange(); });
    expect(api.arrangeSeg?.id).toBe("s1");
  });

  it("deselecting exits, and the button is then inert (nothing remembered)", () => {
    click("s1");
    click(null);
    expect(api.arrangeOn).toBe(false);
    act(() => { api.onArrange(); });
    expect(api.arrangeOn).toBe(false);
  });

  it("selecting something that is not a layout segment exits, like a deselect", () => {
    click("s1");
    click("zoom-1");
    expect(api.arrangeSeg).toBeNull();
  });

  it("entry seeks past the entry transition when the playhead is outside the span", () => {
    click("s1");
    expect(seeks).toEqual([1200]);
  });

  it("entry does NOT seek when the playhead is already inside the span", () => {
    timeMsRef.current = 2000;
    click("s1");
    expect(seeks).toEqual([]);
  });

  it("moving to another segment seeks into THAT one", () => {
    timeMsRef.current = 2000;
    click("s1");
    click("s2");
    expect(seeks).toEqual([4200]); // s2 starts at 4000 and carries the same 200ms entry
  });
});
