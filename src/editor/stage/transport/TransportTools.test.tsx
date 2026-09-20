// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { EditOp } from "../../../shared/edit";
import type { ClickSample } from "../../../shared/ipc";
import type { Range } from "../../timeline/useRangeSelect";
import { actionSpan, TransportTools } from "./TransportTools";

const CLICKS: ClickSample[] = [
  { t: 5000, x: 0, y: 0 },
  { t: 20_000, x: 0, y: 0 },
];

let ops: EditOp[] = [];
let ranges: (Range | null)[] = [];
let silences = 0;
let splits = 0;
let root: Root, container: HTMLDivElement;

const click = (action: string) =>
  act(() => {
    container.querySelector<HTMLElement>(`[data-action="${action}"]`)?.click();
  });
const mount = (range: Range | null, timeMs: number, clicks: ClickSample[], opts: { locked?: boolean } = {}) =>
  act(() => {
    root.render(
      <TransportTools
        locked={opts.locked ?? false}
        trimmed={false}
        onTrimIn={() => {}}
        onTrimOut={() => {}}
        onResetTrim={() => {}}
        onAddZoom={() => {}}
        onSplit={() => {
          splits += 1;
        }}
        onAddText={() => {}}
        onAutoedit={() => {}}
        aiRunning={false}
        exporting={false}
        timeMs={timeMs}
        dur={60_000}
        clicks={clicks}
        range={range}
        setRange={(r) => ranges.push(r)}
        onApply={async (op) => {
          ops.push(op);
          return null;
        }}
        onDetectSilences={() => {
          silences += 1;
        }}
      />,
    );
  });

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  ops = [];
  ranges = [];
  silences = 0;
  splits = 0;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => {
  act(() => {
    root.unmount();
  });
  container.remove();
});

describe("actionSpan", () => {
  it("uses the range when there is one, whatever the playhead and the clicks say", () => {
    expect(actionSpan([1200, 3400], 9000, CLICKS, 60_000)).toEqual([1200, 3400]);
  });
  it("without one, runs from the playhead to the next click within 8 s", () => {
    expect(actionSpan(null, 1000, CLICKS, 60_000)).toEqual([1000, 5000]);
  });
  it("and falls back to a 4 s block when the next click is further off than that", () => {
    expect(actionSpan(null, 1000, [{ t: 20_000, x: 0, y: 0 }], 60_000)).toEqual([1000, 5000]);
    expect(actionSpan(null, 9000, CLICKS, 60_000)).toEqual([9000, 13_000]);
  });
  it("never runs past the clip", () => {
    expect(actionSpan(null, 59_000, [], 60_000)).toEqual([59_000, 60_000]);
  });
});

describe("TransportTools", () => {
  it("Cut with a range applies add_cut for exactly that range, then clears it", () => {
    mount([1200, 3400], 9000, CLICKS);
    click("cut");
    expect(ops).toEqual([{ op: "add_cut", start_ms: 1200, end_ms: 3400 }]);
    expect(ranges).toEqual([null]);
  });

  it("Cut with no range runs to the next click inside the lookahead", () => {
    mount(null, 1000, CLICKS);
    click("cut");
    expect(ops).toEqual([{ op: "add_cut", start_ms: 1000, end_ms: 5000 }]);
  });

  it("Cut with no range and no click that soon takes the 4 s block instead", () => {
    mount(null, 9000, CLICKS);
    click("cut");
    expect(ops).toEqual([{ op: "add_cut", start_ms: 9000, end_ms: 13_000 }]);
  });

  it("Speed applies set_speed at 2x over the same span, and clears the range too", () => {
    mount([1200, 3400], 0, CLICKS);
    click("speed");
    expect(ops).toEqual([{ op: "set_speed", start_ms: 1200, end_ms: 3400, factor: 2 }]);
    expect(ranges).toEqual([null]);
  });
});

describe("the text tool", () => {
  it("is one button beside the zoom tool, and its title names the T shortcut", () => {
    mount(null, 0, []);
    const tools = [...container.querySelectorAll<HTMLElement>("button.e-tg")];
    const text = tools.find((b) => (b.getAttribute("title") ?? "").includes("(T)"));
    expect(text, "no transport tool carries the T shortcut").toBeTruthy();
    expect(text!.getAttribute("title")).toContain("callout");
  });
});

describe("Remove silences", () => {
  it("is one button that hands off to the scan without touching the doc or the range itself", () => {
    mount([1000, 2000], 0, CLICKS);
    click("silences");
    expect(silences).toBe(1);
    expect(ops).toEqual([]);
    expect(ranges).toEqual([]);
  });
});

describe("the split tool", () => {
  it("splits at the playhead and says so", () => {
    mount(null, 4000, []);
    const btn = container.querySelector<HTMLButtonElement>('[data-action="split"]');
    expect(btn?.getAttribute("title")).toBe("Split at the playhead (B)");
    click("split");
    expect(splits).toBe(1);
  });

  it("does not split while the editor is locked", () => {
    mount(null, 0, [], { locked: true });
    expect(container.querySelector('[data-action="split"]')?.hasAttribute("disabled")).toBe(true);
  });
});
