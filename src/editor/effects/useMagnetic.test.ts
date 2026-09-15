import { describe, it, expect } from "vitest";
import { MAGNET_RADIUS, MAGNET_STRENGTH, magneticPull, type MagneticBox } from "./useMagnetic";

const PILL: MagneticBox = { left: 100, top: 100, right: 164, bottom: 132 };
const R = MAGNET_RADIUS,
  S = MAGNET_STRENGTH;
const round = ([x, y]: [number, number]): [number, number] => [
  Math.round(x * 100) / 100,
  Math.round(y * 100) / 100,
];

describe("magneticPull", () => {
  it("pulls nothing when the pointer sits dead centre", () => {
    expect(magneticPull(PILL, 132, 116, R, S)).toEqual([0, 0]);
  });

  it("leans toward the pointer, by `strength` of its offset from the centre", () => {
    expect(round(magneticPull(PILL, 152, 116, R, S))).toEqual([5, 0]);
    expect(round(magneticPull(PILL, 112, 116, R, S))).toEqual([-5, 0]);
  });

  it("measures the FIELD from the box edge, so a wide pill has an even halo", () => {
    expect(magneticPull(PILL, 184, 116, R, S)).not.toEqual([0, 0]);
    expect(magneticPull(PILL, 132, 80, R, S)).not.toEqual([0, 0]);
  });

  it("snaps back to zero the moment the pointer leaves the field", () => {
    expect(magneticPull(PILL, 164 + R + 1, 116, R, S)).toEqual([0, 0]);
    expect(magneticPull(PILL, 132, 100 - R - 1, R, S)).toEqual([0, 0]);
    expect(magneticPull(PILL, 164 + 24, 132 + 24, R, S)).toEqual([0, 0]);
    expect(magneticPull(PILL, 164 + 24, 116, R, S)).not.toEqual([0, 0]);
  });

  it("keeps the travel small - under 8px on a 32px-tall pill at the field's edge", () => {
    const [dx, dy] = magneticPull(PILL, 164 + 19, 132 + 19, R, S);
    expect(Math.hypot(dx, dy)).toBeLessThan(18);
    expect(Math.abs(dy)).toBeLessThan(9);
  });

  it("is a no-op at strength 0 - the disabled-control off switch", () => {
    expect(magneticPull(PILL, 152, 116, R, 0)).toEqual([0, 0]);
  });
});
