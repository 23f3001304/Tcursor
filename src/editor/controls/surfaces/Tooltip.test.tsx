// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { Tooltip, TOOLTIP_DELAY_MS } from "./Tooltip";

let root: Root, container: HTMLDivElement;
const wrap = () => container.querySelector<HTMLElement>(".e-tipwrap")!;

const tip = () => document.querySelector(".e-tip");
const show = () =>
  act(() => {
    root.render(
      <Tooltip label="Background">
        <button>bg</button>
      </Tooltip>,
    );
  });
const fire = (type: string) =>
  act(() => {
    wrap().dispatchEvent(new MouseEvent(type, { bubbles: true }));
  });
const wait = (ms: number) =>
  act(() => {
    vi.advanceTimersByTime(ms);
  });

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  vi.useFakeTimers();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => {
  act(() => {
    root.unmount();
  });
  container.remove();
  vi.useRealTimers();
});

describe("Tooltip", () => {
  it("stays out of the way until the pointer has rested on the control", () => {
    show();
    expect(tip()).toBeNull();
    fire("mouseover");
    wait(TOOLTIP_DELAY_MS - 50);
    expect(tip()).toBeNull();
    wait(50);
    expect(tip()?.textContent).toBe("Background");
  });

  it("never fires for a pointer that only swept across it", () => {
    show();
    fire("mouseover");
    wait(TOOLTIP_DELAY_MS - 100);
    fire("mouseout");
    wait(TOOLTIP_DELAY_MS);
    expect(tip()).toBeNull();
  });

  it("shows at once on keyboard focus, since that control was reached deliberately", () => {
    show();
    act(() => {
      container.querySelector("button")!.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    });
    expect(tip()?.textContent).toBe("Background");
  });

  it("renders the label outside the anchor's own subtree, so no overflow ancestor can clip it", () => {
    show();
    fire("mouseover");
    wait(TOOLTIP_DELAY_MS);
    const el = tip();
    expect(el).not.toBeNull();
    expect(wrap().contains(el)).toBe(false);
    expect(container.querySelector(".e-tip")).toBeNull();
  });

  it("places itself in viewport coordinates and only shows once measured", () => {
    show();
    fire("mouseover");
    wait(TOOLTIP_DELAY_MS);
    const el = tip() as HTMLElement;
    expect(el.style.left).not.toBe("");
    expect(el.style.top).not.toBe("");
    expect(el.style.visibility).toBe("visible");
  });
});
