// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { densityFor, densityVars, sameDensity, MIN_SCALE } from "./density";

describe("densityFor: the pinned ladder", () => {
  it("1366 x 768 sits on the floor and takes both the width and the height steps", () => {
    const d = densityFor(1366, 768);
    expect(d.scale).toBe(0.86);
    expect({ narrow: d.narrow, tight: d.tight, cramped: d.cramped, short: d.short }).toEqual({
      narrow: true,
      tight: true,
      cramped: false,
      short: true,
    });
    expect({ rail: d.rail, railBtn: d.railBtn, panel: d.panel, side: d.side, gutter: d.gutter }).toEqual({
      rail: 48,
      railBtn: 36,
      panel: 288,
      side: 288,
      gutter: 60,
    });
    expect({ film: d.film, row: d.row, audioRow: d.audioRow, gap: d.gap, tracks: d.tracks }).toEqual({
      film: 64,
      row: 28,
      audioRow: 20,
      gap: 5,
      tracks: 168,
    });
  });

  it("1440 x 900 is the reference: scale 1, and nothing steps - the owner's own window is untouched", () => {
    const d = densityFor(1440, 900);
    expect(d.scale).toBe(1);
    expect({ narrow: d.narrow, tight: d.tight, cramped: d.cramped, short: d.short }).toEqual({
      narrow: false,
      tight: false,
      cramped: false,
      short: false,
    });
    expect({ rail: d.rail, panel: d.panel, side: d.side, gutter: d.gutter }).toEqual({
      rail: 56,
      panel: 320,
      side: 360,
      gutter: 72,
    });
    expect({ film: d.film, row: d.row, audioRow: d.audioRow, tracks: d.tracks }).toEqual({
      film: 80,
      row: 32,
      audioRow: 22,
      tracks: 230,
    });
  });

  it("1536 x 864 steps the type only - no column and no lane moves", () => {
    const d = densityFor(1536, 864);
    expect(d.scale).toBe(0.96);
    expect({ narrow: d.narrow, tight: d.tight, cramped: d.cramped, short: d.short }).toEqual({
      narrow: false,
      tight: false,
      cramped: false,
      short: false,
    });
    expect({
      rail: d.rail,
      panel: d.panel,
      side: d.side,
      film: d.film,
      row: d.row,
      tracks: d.tracks,
    }).toEqual({ rail: 56, panel: 320, side: 360, film: 80, row: 32, tracks: 230 });
  });

  it("1920 x 1080 and 2194 x 1234 are both the full design, untouched", () => {
    for (const [w, h] of [
      [1920, 1080],
      [2194, 1234],
    ]) {
      const d = densityFor(w, h);
      expect(d.scale).toBe(1);
      expect([d.narrow, d.tight, d.cramped, d.short]).toEqual([false, false, false, false]);
      expect({ rail: d.rail, railBtn: d.railBtn, panel: d.panel, side: d.side, gutter: d.gutter }).toEqual({
        rail: 56,
        railBtn: 42,
        panel: 320,
        side: 360,
        gutter: 72,
      });
      expect({ film: d.film, row: d.row, audioRow: d.audioRow, gap: d.gap, tracks: d.tracks }).toEqual({
        film: 80,
        row: 32,
        audioRow: 22,
        gap: 6,
        tracks: 230,
      });
    }
  });
});

describe("densityFor: the ends of the ladder", () => {
  it("makes the stage the largest thing on screen at the 880 x 560 minimum window", () => {
    const d = densityFor(880, 560);
    expect(d.cramped).toBe(true);
    const chrome = Math.round(10 * d.scale) * 2 + Math.round(10 * d.scale) * 3;
    const stage = 880 - chrome - d.rail - d.panel - d.side;
    expect(stage).toBeGreaterThan(d.panel);
    expect(stage).toBeGreaterThan(d.side);
  });

  it("never scales below the floor, however small the window gets", () => {
    expect(densityFor(320, 200).scale).toBe(MIN_SCALE);
    expect(densityFor(0, 0).scale).toBe(MIN_SCALE);
  });

  it("never scales above 1, however large", () => {
    expect(densityFor(3840, 2160).scale).toBe(1);
  });

  it("takes the shorter axis: a wide, short window scales by its height", () => {
    expect(densityFor(2400, 792).scale).toBe(0.88);
    expect(densityFor(1296, 1800).scale).toBe(0.9);
  });
});

describe("densityVars", () => {
  it("writes every stepped size as a px custom property, and the scale unitless", () => {
    expect(densityVars(densityFor(1366, 768))).toEqual({
      "--e-density": "0.86",
      "--e-rail-w": "48px",
      "--e-rail-btn": "36px",
      "--e-panel-w": "288px",
      "--e-side-w": "288px",
      "--e-gutter-w": "60px",
      "--e-film-h": "64px",
      "--e-row-h": "28px",
      "--e-audio-h": "20px",
      "--e-row-gap": "5px",
      "--e-tracks-h": "168px",
    });
  });
});

describe("sameDensity", () => {
  it("is true across a resize that crosses no step, false across one that does", () => {
    expect(sameDensity(densityFor(1920, 1080), densityFor(2194, 1234))).toBe(true);
    expect(sameDensity(densityFor(1440, 900), densityFor(1366, 768))).toBe(false);
  });
});
