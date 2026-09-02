import { describe, it, expect } from "vitest";
import { isNaturalPlaybackTick, NATURAL_TICK_MAX_DELTA_MS } from "./playbackTick";

describe("isNaturalPlaybackTick", () => {
  it("is false for ANY change while paused - there is no natural progression to compare against", () => {
    expect(isNaturalPlaybackTick(1000, 1010, false)).toBe(false);
    expect(isNaturalPlaybackTick(1000, 1000, false)).toBe(false);
    expect(isNaturalPlaybackTick(1000, 900, false)).toBe(false);
  });

  it("is true for a small forward delta while playing (a normal ~60ms composite-loop tick)", () => {
    expect(isNaturalPlaybackTick(1000, 1060, true)).toBe(true);
    expect(isNaturalPlaybackTick(1000, 1000, true)).toBe(true); // zero delta is still natural
  });

  it("is true right at the default max delta, false just past it", () => {
    expect(isNaturalPlaybackTick(1000, 1000 + NATURAL_TICK_MAX_DELTA_MS, true)).toBe(true);
    expect(isNaturalPlaybackTick(1000, 1001 + NATURAL_TICK_MAX_DELTA_MS, true)).toBe(false);
  });

  it("is false for a backward jump even while playing - a rewind/loop is a seek, not a tick", () => {
    expect(isNaturalPlaybackTick(1000, 999, true)).toBe(false);
    expect(isNaturalPlaybackTick(5000, 0, true)).toBe(false);
  });

  it("is false for a big forward jump while playing - a scrub without pausing first is still a seek", () => {
    expect(isNaturalPlaybackTick(1000, 10000, true)).toBe(false);
  });

  it("honors a custom maxDeltaMs", () => {
    expect(isNaturalPlaybackTick(1000, 1100, true, 50)).toBe(false);
    expect(isNaturalPlaybackTick(1000, 1040, true, 50)).toBe(true);
  });
});
