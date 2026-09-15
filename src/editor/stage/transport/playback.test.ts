import { describe, expect, it } from "vitest";
import { identityMap } from "../../../shared/math/remap";
import { fixtureMap } from "../../../shared/math/remap.fixture";
import { isCutJump, isNaturalPlaybackTick, NATURAL_TICK_MAX_DELTA_MS, playbackAction } from "./playback";

describe("playbackAction", () => {
  it("jumps to the end of a cut and runs at 1x outside every span", () => {
    expect(playbackAction(fixtureMap(), 1200)).toEqual({ seekTo: 2000, rate: 1 });
    expect(playbackAction(fixtureMap(), 5000)).toEqual({ seekTo: null, rate: 1 });
  });
  it("jumps to the trim-in from before it", () => {
    expect(playbackAction(fixtureMap(), 100).seekTo).toBe(500);
  });
  it("runs at the span's factor inside a speed span", () => {
    expect(playbackAction(fixtureMap(), 3000).rate).toBe(2);
    expect(playbackAction(fixtureMap(), 7000).rate).toBe(0.5);
  });
  it("does not seek at the trim-out edge", () => {
    expect(playbackAction(fixtureMap(), 9500).seekTo).toBeNull();
  });
  it("is the identity on a plain map", () => {
    expect(playbackAction(identityMap(10_000), 4321)).toEqual({ seekTo: null, rate: 1 });
  });
});

describe("isCutJump", () => {
  it("recognises the preview landing on a cut's end from just before or inside its start", () => {
    expect(isCutJump(fixtureMap(), 990, 2000)).toBe(true);
    expect(isCutJump(fixtureMap(), 1020, 2010)).toBe(true);
  });
  it("treats any other long jump as a seek", () => {
    expect(isCutJump(fixtureMap(), 990, 3000)).toBe(false);
    expect(isCutJump(fixtureMap(), 100, 2000)).toBe(false);
    expect(isCutJump(identityMap(10_000), 990, 2000)).toBe(false);
  });
});

describe("isNaturalPlaybackTick", () => {
  it("is false for ANY change while paused - there is no natural progression to compare against", () => {
    expect(isNaturalPlaybackTick(1000, 1010, false)).toBe(false);
    expect(isNaturalPlaybackTick(1000, 1000, false)).toBe(false);
    expect(isNaturalPlaybackTick(1000, 900, false)).toBe(false);
  });

  it("is true for a small forward delta while playing (a normal ~60ms composite-loop tick)", () => {
    expect(isNaturalPlaybackTick(1000, 1060, true)).toBe(true);
    expect(isNaturalPlaybackTick(1000, 1000, true)).toBe(true);
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
