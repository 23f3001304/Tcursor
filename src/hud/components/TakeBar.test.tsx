import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { TakeBar } from "./TakeBar";

// jsdom has no matchMedia; the wave's reduced-motion hook and Motion both ask for it.
vi.stubGlobal("matchMedia", () => ({ matches: false, addEventListener() {}, removeEventListener() {}, addListener() {}, removeListener() {} }));

const base: Parameters<typeof TakeBar>[0] = {
  paused: false, saving: false, savePct: 0, elapsed: 12_000, err: null, micOn: true, sysOn: false, live: true,
  read: () => ({ mic: 0, sys: 0 }), camRef: () => {}, camOn: false, camLive: false,
  toggle: vi.fn(), togglePause: vi.fn(), sources: false, onSources: vi.fn(),
};

let host: HTMLDivElement; let root: Root;
beforeEach(() => { host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host); });
afterEach(() => { act(() => root.unmount()); host.remove(); vi.clearAllMocks(); });
const render = (p: Partial<typeof base>) => act(() => root.render(<TakeBar {...base} {...p} />));
const button = (label: string) => host.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`);

describe("TakeBar", () => {
  it("shows the timer, Sources, a Pause and a Stop, and no window buttons", () => {
    render({});
    expect(host.querySelector(".take-timer")?.textContent).toBe("0:12");
    expect(button("Sources")).not.toBeNull();
    expect(button("Pause")).not.toBeNull();
    expect(button("Stop and save")).not.toBeNull();
    expect(host.querySelectorAll("button")).toHaveLength(3);
    expect(host.querySelector(".take-slot svg")).not.toBeNull();
  });

  // Mid-take source switching (2026-09-14): the one affordance the pill grew, left of Pause, and
  // it says whether the sheet under the pill is open.
  it("Sources sits left of Pause and reports whether its sheet is open", () => {
    render({});
    const order = [...host.querySelectorAll("button")].map((b) => b.getAttribute("aria-label"));
    expect(order).toEqual(["Sources", "Pause", "Stop and save"]);
    expect(button("Sources")!.getAttribute("aria-expanded")).toBe("false");
    act(() => button("Sources")!.click());
    expect(base.onSources).toHaveBeenCalledTimes(1);
    render({ sources: true });
    expect(button("Sources")!.getAttribute("aria-expanded")).toBe("true");
    expect(button("Sources")!.classList.contains("on")).toBe(true);
  });

  it("Stop stops and Pause pauses", () => {
    render({});
    act(() => button("Stop and save")!.click());
    act(() => button("Pause")!.click());
    expect(base.toggle).toHaveBeenCalledTimes(1);
    expect(base.togglePause).toHaveBeenCalledTimes(1);
  });

  it("paused: the word Paused takes the meter's slot and the button offers Resume", () => {
    render({ paused: true });
    expect(host.querySelector(".take")?.classList.contains("paused")).toBe(true);
    expect(host.querySelector(".take-slot")?.textContent).toBe("Paused");
    expect(button("Resume")).not.toBeNull();
    expect(button("Pause")).toBeNull();
  });

  it("a source that is off is not in the pill at all", () => {
    render({ micOn: false, sysOn: false, camOn: false });
    expect(host.querySelector(".take-slot")).toBeNull();
    expect(host.querySelector(".camtoggle")).toBeNull();
    render({ micOn: false, sysOn: true, camOn: true });
    expect(host.querySelector(".take-slot svg")).not.toBeNull();
    expect(host.querySelector(".camtoggle.round")).not.toBeNull();
  });

  it("a mid-take warning rides on a hover, not in a title bar", () => {
    render({ err: "Webcam stopped" });
    expect(host.querySelector(".take-warn")?.getAttribute("title")).toBe("Webcam stopped");
  });

  it("saving: the word, the percentage and no buttons at all", () => {
    render({ saving: true, savePct: 43 });
    expect(host.querySelector(".take")?.classList.contains("saving")).toBe(true);
    expect(host.querySelector(".take-label")?.textContent).toBe("Saving");
    expect(host.querySelector(".take-pct")?.textContent).toBe("43%");
    expect(host.querySelectorAll("button")).toHaveLength(0);
  });
});
