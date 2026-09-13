import { describe, expect, it } from "vitest";
import { identityMap } from "../../lib/remap";
import { fixtureMap } from "../../lib/remap.fixture";
import { isCutJump, playbackAction } from "./playbackRemap";

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
