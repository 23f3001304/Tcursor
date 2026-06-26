import { describe, it, expect } from "vitest";
import { pickDefaults, type DeviceState } from "./selectDevices";

describe("pickDefaults", () => {
  it("selects the first display and mic when present", () => {
    const s: DeviceState = pickDefaults(
      [{ id: 0, label: "Primary Display" }],
      [{ id: "0", label: "Realtek" }, { id: "1", label: "USB Mic" }],
    );
    expect(s.displayId).toBe(0);
    expect(s.micId).toBe("0");
  });
  it("leaves mic null when no inputs", () => {
    const s = pickDefaults([{ id: 0, label: "Primary" }], []);
    expect(s.micId).toBeNull();
  });
});
