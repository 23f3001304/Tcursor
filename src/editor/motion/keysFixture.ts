import { expect } from "vitest";
import { parseKeys, type Keys } from "./keys";

/** The progress samples the Rust parity table is measured at. */
export const P = [0, 0.1, 0.25, 0.5, 0.75, 0.9, 1];

const C = (name: string, wire: string, canon: string, want: number[]) => ({ name, wire, canon, want });

/** The parity table: each curve in wire form, canonical form, and its values at P. */
export const CURVES = [
  C(
    "snappy",
    "keys(0 0 0 0 0.1 0.7 b,1 1 -0.4 0 0 0 b)",
    "keys(0.000 0.000 0.000 0.000 0.100 0.700 b,1.000 1.000 -0.400 0.000 0.000 0.000 b)",
    [0.000001, 0.363633, 0.617844, 0.847027, 0.962607, 0.99391, 1],
  ),
  C(
    "cinematic in",
    "keys(0 0 0 0 0.45 0 b,1 1 -0.25 0 0 0 b)",
    "keys(0.000 0.000 0.000 0.000 0.450 0.000 b,1.000 1.000 -0.250 0.000 0.000 0.000 b)",
    [0, 0.016435, 0.101738, 0.388135, 0.771125, 0.953911, 1],
  ),
  C(
    "cinematic out",
    "keys(0 0 0 0 0.15 -0.06 b,1 1 -0.4 0 0 0 b)",
    "keys(0.000 0.000 0.000 0.000 0.150 -0.060 b,1.000 1.000 -0.400 0.000 0.000 0.000 b)",
    [0, 0.054953, 0.251181, 0.606571, 0.88454, 0.979511, 1],
  ),
  C(
    "mechanical",
    "keys(0 0 0 0 0 0 l,0.85 1 0 0 0 0 h,1 1 0 0 0 0 l)",
    "keys(0.000 0.000 0.000 0.000 0.000 0.000 l,0.850 1.000 0.000 0.000 0.000 0.000 h,1.000 1.000 0.000 0.000 0.000 0.000 l)",
    [0, 0.117647, 0.294118, 0.588235, 0.882353, 1, 1],
  ),
  C(
    "soft",
    "keys(0 0 0 0 0.333 0 b,1 1 -0.333 0 0 0 b)",
    "keys(0.000 0.000 0.000 0.000 0.333 0.000 b,1.000 1.000 -0.333 0.000 0.000 0.000 b)",
    [0, 0.028039, 0.156356, 0.499999, 0.843644, 0.971961, 1],
  ),
  C(
    "hand-made",
    "keys(0 0 0 0 0.15 0.5 b,0.5 0.8 -0.1 0 0 0 h,0.7 0.8 0 0 0 0 l,0.85 1.15 0 0 0.05 0 b,1 1 -0.05 0 0 0 b)",
    "keys(0.000 0.000 0.000 0.000 0.150 0.500 b,0.500 0.800 -0.100 0.000 0.000 0.000 h,0.700 0.800 0.000 0.000 0.000 0.000 l," +
      "0.850 1.150 0.000 0.000 0.050 0.000 b,1.000 1.000 -0.050 0.000 0.000 0.000 b)",
    [0.000001, 0.2752, 0.559285, 0.8, 0.916667, 1.111111, 1],
  ),
];

/** Parses a curve that the test asserts must parse. */
export const parsed = (s: string): Keys => {
  const k = parseKeys(s);
  expect(k).not.toBeNull();
  return k!;
};

export const key = (
  t: number,
  v: number,
  out: [number, number] = [0, 0],
  inn: [number, number] = [0, 0],
) => ({
  t,
  v,
  in: inn,
  out,
  mode: "b" as const,
});
