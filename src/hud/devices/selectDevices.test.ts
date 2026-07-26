import { describe, it, expect } from "vitest";
import { pickDefaults, parseTarget, type DeviceState } from "./selectDevices";

describe("pickDefaults", () => {
  it("selects the first display and mic when present", () => {
    const s: DeviceState = pickDefaults(
      [{ id: "display:0", label: "Primary Display" }],
      [{ id: "0", label: "Realtek" }, { id: "1", label: "USB Mic" }],
    );
    expect(s.displayId).toBe("display:0");
    expect(s.micId).toBe("0");
  });
  it("leaves mic null when no inputs", () => {
    const s = pickDefaults([{ id: "display:0", label: "Primary" }], []);
    expect(s.micId).toBeNull();
  });
});

describe("parseTarget", () => {
  it("marks index 0 primary and strips the (Primary) suffix", () => {
    const r = parseTarget({ id: "display:0", label: "Display 1: \\\\.\\DISPLAY1 (Primary)", kind: "display" }, 0);
    expect(r).toEqual({ title: "Display 1: \\\\.\\DISPLAY1", resolution: null, primary: true });
  });
  it("extracts a resolution for a non-primary display", () => {
    const r = parseTarget({ id: "display:1", label: "Display 2: \\\\.\\DISPLAY2 (1920x1080)", kind: "display" }, 1);
    expect(r).toEqual({ title: "Display 2: \\\\.\\DISPLAY2", resolution: "1920x1080", primary: false });
  });
  it("never marks a window primary even at index 0", () => {
    const r = parseTarget({ id: "window:0x1", label: "App: Notepad", kind: "window" }, 0);
    expect(r).toEqual({ title: "App: Notepad", resolution: null, primary: false });
  });
  it("passes through a label with no parenthetical suffix untouched", () => {
    const r = parseTarget({ id: "display:2", label: "Display 3: \\\\.\\DISPLAY3", kind: "display" }, 2);
    expect(r).toEqual({ title: "Display 3: \\\\.\\DISPLAY3", resolution: null, primary: false });
  });
});
