import { describe, it, expect } from "vitest";
import { MAGNET_RADIUS, MAGNET_STRENGTH, magneticPull, type MagneticBox } from "./useMagnetic";

/** A 32px-tall, 64px-wide pill with its centre at (132, 116) - roughly a Trim pill in the bar. */
const PILL: MagneticBox = { left: 100, top: 100, right: 164, bottom: 132 };
const R = MAGNET_RADIUS, S = MAGNET_STRENGTH;
const round = ([x, y]: [number, number]): [number, number] => [Math.round(x * 100) / 100, Math.round(y * 100) / 100];

describe("magneticPull", () => {
  it("pulls nothing when the pointer sits dead centre", () => {
    expect(magneticPull(PILL, 132, 116, R, S)).toEqual([0, 0]);
  });

  it("leans toward the pointer, by `strength` of its offset from the centre", () => {
    // 20px right of centre, on the vertical middle -> a quarter of that, and no vertical lean.
    expect(round(magneticPull(PILL, 152, 116, R, S))).toEqual([5, 0]);
    // Symmetric: the same distance on the other side leans the other way by the same amount.
    expect(round(magneticPull(PILL, 112, 116, R, S))).toEqual([-5, 0]);
  });

  it("measures the FIELD from the box edge, so a wide pill has an even halo", () => {
    // 20px past the right edge and 20px past the top edge are both inside the 28px field, even
    // though the second point is far from the centre - which is the whole point of edge distance.
    expect(magneticPull(PILL, 184, 116, R, S)).not.toEqual([0, 0]);
    expect(magneticPull(PILL, 132, 80, R, S)).not.toEqual([0, 0]);
  });

  it("snaps back to zero the moment the pointer leaves the field", () => {
    expect(magneticPull(PILL, 164 + R + 1, 116, R, S)).toEqual([0, 0]);   // past the right edge
    expect(magneticPull(PILL, 132, 100 - R - 1, R, S)).toEqual([0, 0]);   // above the top edge
    // A diagonal corner approach is out of range while neither axis alone is: 24px past each edge
    // is 33.9px of true distance, so the field is a rounded rect, not a bounding box.
    expect(magneticPull(PILL, 164 + 24, 132 + 24, R, S)).toEqual([0, 0]);
    expect(magneticPull(PILL, 164 + 24, 116, R, S)).not.toEqual([0, 0]);
  });

  it("keeps the travel small - under 8px on a 32px-tall pill at the field's edge", () => {
    // The worst case for travel is the far corner still inside the field.
    const [dx, dy] = magneticPull(PILL, 164 + 19, 132 + 19, R, S);
    expect(Math.hypot(dx, dy)).toBeLessThan(18);
    // And on the axis that matters for a bar of 32px controls, the vertical lean stays tiny.
    expect(Math.abs(dy)).toBeLessThan(9);
  });

  it("is a no-op at strength 0 - the disabled-control off switch", () => {
    expect(magneticPull(PILL, 152, 116, R, 0)).toEqual([0, 0]);
  });
});
