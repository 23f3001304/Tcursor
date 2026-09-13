# src/lib/remap.fixture.ts

The parity fixture shared by `remap.test.ts`, `remapDoc.test.ts` and `playbackRemap.test.ts`. Test-only; not imported by app code.

## fixtureMap

```ts
export function fixtureMap(): TimeMap
```

The Rust fixture (`export/remap_tests.rs::fixture`) rebuilt in TS: trim 500..9000 of a 10000 ms clip, cuts [1000,2000) and [4000,4500), 2x on [2500,3500), 0.5x on [6000,8000). Every expected value in those tests was computed from this exact map on the Rust side; a TS result that disagrees is wrong, not the table.
