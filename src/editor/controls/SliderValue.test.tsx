import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { parseSliderInput } from "./SliderValue";
import { Slider } from "./Slider";

describe("parseSliderInput", () => {
  it("reads a bare number, snapped and clamped to the slider's own range", () => {
    expect(parseSliderInput("65", 0, 100, 5)).toBe(65);
    expect(parseSliderInput("63", 0, 100, 5)).toBe(65);
    expect(parseSliderInput("900", 0, 100, 5)).toBe(100);
    expect(parseSliderInput("-4", 0, 100, 5)).toBe(0);
  });

  it("tolerates the unit the formatter printed, so retyping it is not an error", () => {
    expect(parseSliderInput("35 %", 0, 100, 5)).toBe(35);
    expect(parseSliderInput("1.20x", 0.4, 3, 0.1)).toBeCloseTo(1.2, 10);
    expect(parseSliderInput("160 deg", 0, 360, 5)).toBe(160);
    expect(parseSliderInput("-40 ms", -300, 300, 10)).toBe(-40);
  });

  it("returns null for text holding no number at all, so the caller writes nothing", () => {
    expect(parseSliderInput("", 0, 100, 5)).toBeNull();
    expect(parseSliderInput("abc", 0, 100, 5)).toBeNull();
  });
});

let root: Root, container: HTMLDivElement;
const q = <T extends HTMLElement>(s: string) => container.querySelector<T>(s);
const key = (el: HTMLElement, k: string) =>
  act(() => { el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true })); });

let committed: number[] = [];
const show = (value: number) => act(() => {
  root.render(
    <Slider label="Background Blur" value={value} min={0} max={100} step={5}
      ariaLabel="Background Blur" formatValue={(v) => `${v}%`}
      onChange={(v) => committed.push(v)} />
  );
});

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  committed = [];
});
afterEach(() => { act(() => { root.unmount(); }); container.remove(); });

// The readout is the only way to set an exact number on a 320px track, so its commit/revert
// behaviour is the part worth pinning: everything else about it is paint.
describe("Slider's editable value readout", () => {
  it("shows the formatted value on the label's own row", () => {
    show(40);
    expect(q(".e-fl")?.textContent).toBe("Background Blur 40%");
  });

  it("commits a typed value on Enter, snapped to the slider's step", () => {
    show(40);
    act(() => { q<HTMLButtonElement>(".e-val")!.click(); });
    const input = q<HTMLInputElement>(".e-val-edit")!;
    expect(input.value).toBe("40"); // opens on the bare number, not the formatted one
    input.value = "63";
    key(input, "Enter");
    expect(committed).toEqual([65]);
    expect(q(".e-val-edit")).toBeNull(); // back to the readout
  });

  it("reverts on Esc: nothing is committed and the old value is still shown", () => {
    show(40);
    act(() => { q<HTMLButtonElement>(".e-val")!.click(); });
    const input = q<HTMLInputElement>(".e-val-edit")!;
    input.value = "90";
    key(input, "Escape");
    expect(committed).toEqual([]);
    expect(q(".e-val")?.textContent).toBe("40%");
  });

  it("commits on blur, so clicking away is not a silent discard", () => {
    show(40);
    act(() => { q<HTMLButtonElement>(".e-val")!.click(); });
    const input = q<HTMLInputElement>(".e-val-edit")!;
    input.value = "20";
    // React delegates `onBlur` off the bubbling `focusout`, not the non-bubbling `blur`.
    act(() => { input.dispatchEvent(new FocusEvent("focusout", { bubbles: true })); });
    expect(committed).toEqual([20]);
  });
});
