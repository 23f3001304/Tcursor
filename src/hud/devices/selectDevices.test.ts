import { describe, it, expect } from "vitest";
import {
  pickDefaults, parseTarget, resolveSelection, cleanDeviceLabel, isOwnProcessWindow,
  prettifyWindowLabel, type DeviceState,
} from "./selectDevices";

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

describe("resolveSelection", () => {
  const displays = [{ id: "display:0", label: "Primary" }];
  const mics = [{ id: "0", label: "Realtek" }, { id: "1", label: "USB Mic" }];

  it("keeps the current pick when it is still present", () => {
    const s = resolveSelection({ displayId: "display:0", micId: "1" }, displays, mics);
    expect(s).toEqual({ displayId: "display:0", micId: "1" });
  });

  it("falls back to the first mic when the picked one disappeared (unplugged)", () => {
    const s = resolveSelection({ displayId: "display:0", micId: "gone" }, displays, mics);
    expect(s.micId).toBe("0");
  });

  it("falls back to null when the picked device disappeared and none remain", () => {
    const s = resolveSelection({ displayId: "display:0", micId: "gone" }, displays, []);
    expect(s.micId).toBeNull();
  });

  it("resolves an initial (null) selection the same way pickDefaults does", () => {
    const s = resolveSelection({ displayId: null, micId: null }, displays, mics);
    expect(s).toEqual(pickDefaults(displays, mics));
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
  it("prettifies a window titled with a raw filesystem path to its basename, no extension", () => {
    const r = parseTarget({ id: "window:0x2", label: "App: C:\\Users\\coehe\\AppData\\Local\\TCursor\\tcursor-scaffold.exe", kind: "window" }, 3);
    expect(r).toEqual({ title: "App: tcursor-scaffold", resolution: null, primary: false });
  });
});

describe("cleanDeviceLabel", () => {
  it("unwraps a Windows category-wrapped device name with cpal's numeric prefix", () => {
    expect(cleanDeviceLabel("Microphone (3- Insta360 Link 2C)")).toBe("Insta360 Link 2C");
  });
  it("preserves nested parens in the unwrapped product name", () => {
    expect(cleanDeviceLabel("Microphone (Realtek(R) Audio)")).toBe("Realtek(R) Audio");
  });
  it("unwraps a Headset-category label", () => {
    expect(cleanDeviceLabel("Headset (WH-1000XM4 Hands-Free AG Audio)")).toBe("WH-1000XM4 Hands-Free AG Audio");
  });
  it("strips a numeric enumeration prefix with no category wrapper", () => {
    expect(cleanDeviceLabel("2- USB Audio Device")).toBe("USB Audio Device");
  });
  it("drops the Windows Virtual Camera driver-shim suffix", () => {
    expect(cleanDeviceLabel("Insta360 Link 2C (Windows Virtual Camera)")).toBe("Insta360 Link 2C");
  });
  it("leaves an already-clean label untouched", () => {
    expect(cleanDeviceLabel("Realtek(R) Audio")).toBe("Realtek(R) Audio");
  });
  it("unwraps a two-word 'Headset Microphone' category (Bluetooth/USB headset mic endpoint)", () => {
    expect(cleanDeviceLabel("Headset Microphone (2- Realtek(R) Audio)")).toBe("Realtek(R) Audio");
  });
  it("unwraps a two-word 'Microphone Array' category (built-in laptop array mic)", () => {
    expect(cleanDeviceLabel("Microphone Array (Realtek High Definition Audio)")).toBe("Realtek High Definition Audio");
  });
  it("unwraps a two-word 'Headset Earphone' category", () => {
    expect(cleanDeviceLabel("Headset Earphone (2- Realtek(R) Audio)")).toBe("Realtek(R) Audio");
  });
  it("leaves a two-word category name with no parenthetical untouched", () => {
    expect(cleanDeviceLabel("Headset Microphone")).toBe("Headset Microphone");
  });
});

describe("isOwnProcessWindow", () => {
  it("flags this process's raw-path window", () => {
    expect(isOwnProcessWindow({ id: "window:0x1", label: "App: C:\\Users\\x\\tcursor-scaffold.exe", kind: "window" })).toBe(true);
  });
  it("does not flag an unrelated window", () => {
    expect(isOwnProcessWindow({ id: "window:0x2", label: "App: Notepad", kind: "window" })).toBe(false);
  });
  it("does not flag a display, even if its label matched the pattern by coincidence", () => {
    expect(isOwnProcessWindow({ id: "display:0", label: "tcursor-scaffold.exe", kind: "display" })).toBe(false);
  });
});

describe("prettifyWindowLabel", () => {
  it("reduces a raw exe path title to its basename with no extension", () => {
    expect(prettifyWindowLabel("App: C:\\Users\\coehe\\AppData\\Local\\TCursor\\tcursor-scaffold.exe")).toBe("App: tcursor-scaffold");
  });
  it("leaves a normal window title untouched", () => {
    expect(prettifyWindowLabel("App: Notepad")).toBe("App: Notepad");
  });
});
