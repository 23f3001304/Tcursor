import { describe, it, expect } from "vitest";
import { gradeFragmentSource } from "./gradeShader";
import { CORNER, LUMA, TEMP_GAIN, TINT_GAIN, VIGN_IN } from "./gradeParams";

describe("gradeShader", () => {
  const src = gradeFragmentSource();

  it("declares GLSL ES 3.0 and one output", () => {
    expect(src.startsWith("#version 300 es")).toBe(true);
    expect(src).toContain("out vec4");
  });

  it("carries the same constants gradeParams.ts does, so they cannot drift", () => {
    for (const n of [LUMA[0], LUMA[1], LUMA[2], VIGN_IN, CORNER, TEMP_GAIN, TINT_GAIN]) {
      expect(src, `constant ${n} is missing from the shader`).toContain(String(n));
    }
  });

  it("keeps the eight steps in the order the Rust reference fixes", () => {
    const order = ["exp2", "lift", "pow", "contrast", "luma", "vignette"];
    let at = -1;
    for (const token of order) {
      const next = src.indexOf(token, at + 1);
      expect(next, `${token} is missing or out of order`).toBeGreaterThan(at);
      at = next;
    }
  });
});
