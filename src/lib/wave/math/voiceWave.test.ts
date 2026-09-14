import { describe, it, expect } from "vitest";
import { AMP_MAX, AMP_MIN, CEIL_DB, IDLE_DB, heightFromRms } from "./level";
import { AMP_CEIL_PX, AMP_FLOOR_PX, AMP_RELEASE_S, IDLE_AFTER_S, IDLE_SWELL_PX, LAYERS, MAX_DT_S, ceilFor,
  idleAmp, initialVoiceState, layerPath, layerSpec, lensPath, taperWindow, voiceFrame, type VoiceState } from "./voiceWave";

const DT = 1 / 60, W = 104, H = 30;
const SILENT = Math.pow(10, (IDLE_DB - 10) / 20), LOUD = 0.5;   // 10 dB under the idle floor; ~-6 dBFS

/** Run `seconds` worth of frames at `dt`, feeding the same levels every frame. */
function run(s: VoiceState, mic: number, sys: number, seconds: number, dt = DT, ceil?: number): VoiceState {
  const n = Math.round(seconds / dt);
  for (let i = 0; i < n; i++) s = voiceFrame(s, mic, sys, dt, ceil);
  return s;
}

/** Every "x y" pair in a path `d`, as numbers. */
function points(d: string): Array<[number, number]> {
  const nums = d.replace(/[MLZ]/g, " ").trim().split(/\s+/).map(Number);
  const out: Array<[number, number]> = [];
  for (let i = 0; i + 1 < nums.length; i += 2) out.push([nums[i], nums[i + 1]]);
  return out;
}

describe("taperWindow", () => {
  it("is zero at both ends and flat at full amplitude across the middle", () => {
    expect(taperWindow(0)).toBe(0);
    expect(taperWindow(1)).toBe(0);
    for (const t of [0.4, 0.5, 0.6]) expect(taperWindow(t)).toBe(1);
  });

  it("rises monotonically through the taper and is symmetric about the centre", () => {
    let prev = -1;
    for (let t = 0; t <= 0.5; t += 0.01) {
      const v = taperWindow(t);
      expect(v).toBeGreaterThanOrEqual(prev - 1e-12);
      expect(v).toBeCloseTo(taperWindow(1 - t), 10);
      prev = v;
    }
  });
});

describe("LAYERS", () => {
  it("keeps every multiplier inside the design's stated ranges", () => {
    expect(LAYERS).toHaveLength(4);
    for (const l of LAYERS) {
      expect(l.freq).toBeGreaterThanOrEqual(0.8);
      expect(l.freq).toBeLessThanOrEqual(1.6);
      expect(l.amp).toBeGreaterThanOrEqual(0.5);
      expect(l.amp).toBeLessThanOrEqual(1.0);
      expect(l.alpha).toBeGreaterThanOrEqual(0.4);
      expect(l.alpha).toBeLessThanOrEqual(0.6);
    }
  });

  it("paints the widest and most opaque layer first, and sends neighbours opposite ways", () => {
    expect(LAYERS[0].amp).toBe(1);
    expect(new Set(LAYERS.map((l) => l.phase)).size).toBe(LAYERS.length);
    for (let i = 1; i < LAYERS.length; i++) {
      expect(LAYERS[i].alpha).toBeLessThan(LAYERS[i - 1].alpha);
      expect(LAYERS[i].amp).toBeLessThan(LAYERS[i - 1].amp);
      // opposite drift signs are what make the layers cross rather than travel in formation
      expect(Math.sign(LAYERS[i].drift)).not.toBe(Math.sign(LAYERS[i - 1].drift));
    }
  });
});

describe("voiceFrame level mapping", () => {
  const at = (mic: number, sys = 0) => run(initialVoiceState(false), mic, sys, 0.5).amp;

  it("is a flat line at silence and full height from the ceiling up, half of level.ts's peak-to-peak", () => {
    expect(at(0)).toBeCloseTo(AMP_FLOOR_PX, 1);
    expect(at(Math.pow(10, CEIL_DB / 20))).toBeCloseTo(AMP_CEIL_PX, 1);
    expect(at(1)).toBeCloseTo(AMP_CEIL_PX, 1);
    expect([AMP_FLOOR_PX, AMP_CEIL_PX]).toEqual([AMP_MIN / 2, AMP_MAX / 2]);
  });

  it("is log-mapped inside the speech window: each halving of loudness costs the same number of px", () => {
    const dropA = at(0.1) - at(0.05);   // -20 -> -26 dBFS
    expect(dropA).toBeGreaterThan(1);
    expect(at(0.05) - at(0.025)).toBeCloseTo(dropA, 1);
  });

  it("raises its ceiling with the slot: a 40px slot reaches 17px, the 30px design stays at 12", () => {
    expect([ceilFor(30), ceilFor(40)]).toEqual([AMP_CEIL_PX, 17]);
    expect(run(initialVoiceState(false), 1, 0, 0.5, DT, ceilFor(40)).amp).toBeCloseTo(17, 1);
  });

  it("follows whichever source is louder, so system audio drives the wave too", () => {
    expect(at(LOUD)).toBeCloseTo(heightFromRms(LOUD) / 2, 1);
    expect(at(0, LOUD)).toBeCloseTo(heightFromRms(LOUD) / 2, 1);
  });
});

describe("voiceFrame ballistics", () => {
  it("attacks faster than it releases", () => {
    const quiet = initialVoiceState(false);
    const risen = run(quiet, LOUD, 0, 0.05).amp - quiet.amp;
    const loud = run(quiet, LOUD, 0, 0.5);
    const fallen = loud.amp - run(loud, 0, 0, 0.05).amp;
    expect(risen).toBeGreaterThan(fallen);
  });

  it("takes about 300ms to fall back to the line, not one frame and not a second", () => {
    const loud = run(initialVoiceState(false), LOUD, 0, 0.5);
    const swing = loud.amp - AMP_FLOOR_PX;
    // 4.744 tau is where a critically damped step has 5% of its swing left, so this IS the
    // ~300ms the release constant claims - asserted with slack for the frame quantisation.
    expect(run(loud, 0, 0, AMP_RELEASE_S * 4.744).amp - AMP_FLOOR_PX).toBeLessThan(swing * 0.08);
    // and one lag constant in it is still most of the way up, so the decay is visible, not a cut
    expect(run(loud, 0, 0, AMP_RELEASE_S).amp - AMP_FLOOR_PX).toBeGreaterThan(swing * 0.5);
  });

  it("lands in the same place whatever the frame rate", () => {
    // `n` frames of `0.25 / n` seconds each, so both runs cover EXACTLY the same quarter second
    // and the comparison is about the solver rather than about loop rounding.
    const over = (n: number) => run(initialVoiceState(false), LOUD, 0, 0.25, 0.25 / n).amp;
    expect(over(36)).toBeCloseTo(over(5), 9);
  });

  it("clamps a stalled frame instead of teleporting the clock", () => {
    const s = voiceFrame(initialVoiceState(false), LOUD, 0, 30);
    expect(s.t).toBe(MAX_DT_S);
    expect(s.amp).toBeLessThanOrEqual(AMP_CEIL_PX);
  });
});

describe("voiceFrame idle breath", () => {
  it("stays dead flat through a short pause and only breathes after the idle delay", () => {
    const brief = run(initialVoiceState(false), SILENT, SILENT, IDLE_AFTER_S * 0.5);
    expect(brief.amp).toBeCloseTo(AMP_FLOOR_PX, 2);
    const amps: number[] = [];
    let s = run(brief, SILENT, SILENT, IDLE_AFTER_S);
    for (let i = 0; i < Math.round(2 / DT); i++) { s = voiceFrame(s, SILENT, SILENT, DT); amps.push(s.amp); }
    expect(Math.max(...amps) - Math.min(...amps)).toBeGreaterThan(IDLE_SWELL_PX * 0.5);
  });

  it("keeps the breath inside the 1 to 2 px window a resting wave is allowed, and drops it the instant sound arrives", () => {
    for (let t = 0; t < 4; t += 0.05) {
      expect(idleAmp(t)).toBeGreaterThanOrEqual(1);
      expect(idleAmp(t)).toBeLessThanOrEqual(2);
    }
    const idle = run(initialVoiceState(false), SILENT, SILENT, IDLE_AFTER_S + 0.5);
    expect(voiceFrame(idle, LOUD, 0, DT).quietS).toBe(0);
  });
});

describe("voiceFrame under reduced motion", () => {
  it("freezes the clock, so the layers never drift", () => {
    const s = run(initialVoiceState(true), LOUD, 0, 1);
    expect(s.t).toBe(0);
    expect(layerPath(s, LAYERS[0], W, H)).toBe(layerPath(run(s, LOUD, 0, 1), LAYERS[0], W, H));
  });

  it("still shows the level, snapped straight to it with no ballistics", () => {
    const s = voiceFrame(initialVoiceState(true), LOUD, 0, DT);
    expect(s.amp).toBeCloseTo(heightFromRms(LOUD) / 2, 10);
    expect(s.ampV).toBe(0);
  });
});

describe("lensPath", () => {
  it("is a closed outline that spans the full width", () => {
    const d = layerPath(run(initialVoiceState(false), LOUD, 0, 0.5), LAYERS[0], W, H);
    expect(d.startsWith("M ")).toBe(true);
    expect(d.endsWith(" Z")).toBe(true);
    const xs = points(d).map((p) => p[0]);
    expect(Math.min(...xs)).toBe(0);
    expect(Math.max(...xs)).toBe(W);
  });

  it("mirrors every sample about the midline", () => {
    const spec = layerSpec(run(initialVoiceState(false), LOUD, 0, 0.5), LAYERS[0], W, H);
    const pts = points(lensPath(spec));
    const half = pts.length / 2;
    for (let i = 0; i < half; i++) {
      const [xTop, yTop] = pts[i];
      const [xBot, yBot] = pts[pts.length - 1 - i];
      expect(xBot).toBeCloseTo(xTop, 6);
      expect((yTop + yBot) / 2).toBeCloseTo(spec.mid, 6);
    }
  });

  it("pins both ends onto the centre line, whatever the level", () => {
    for (const rms of [0, SILENT, LOUD, 1]) {
      const pts = points(layerPath(run(initialVoiceState(false), rms, 0, 0.5), LAYERS[1], W, H));
      expect(pts[0][1]).toBeCloseTo(H / 2, 2);
      expect(pts[pts.length - 1][1]).toBeCloseTo(H / 2, 2);
    }
  });

  it("never draws outside the meter's box at full scale", () => {
    const loud = run(initialVoiceState(false), 1, 1, 1);
    for (const layer of LAYERS) {
      for (const [, y] of points(layerPath(loud, layer, W, H))) {
        expect(y).toBeGreaterThanOrEqual(0);
        expect(y).toBeLessThanOrEqual(H);
      }
    }
  });
});
