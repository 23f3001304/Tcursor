# Bundled cursor packs

One folder per pack, in the same shape an imported pack uses (`export/cursor/pack.rs`): nine
sprites named by `CursorType` wire name (`arrow`, `ibeam`, `hand`, `resize_ns`, `resize_ew`,
`resize_nwse`, `resize_nesw`, `move`, `busy`), `hotspots.json` (`{kind: [hx, hy]}` as canvas
fractions) and `pack.json`. Sprites are 128x128 RGBA PNGs with straight alpha.

`pack.json` `version: 2` adds `busy: { anim, fps }` - the busy sprite is a single still and the
renderer synthesises the animation from it: `spin` rotates a full turn per cycle (rings,
spinners), `flip` holds then turns 180 degrees (hourglasses), `pulse` breathes 1.0 -> 1.06 -> 1.0
(sleeping cat, pocket watch). A pack may instead ship explicit `busy_00.png ... busy_NN.png`
frames, which take precedence when present.

All packs were generated for TCursor on 2026-09-13 with OpenAI `gpt-image-2.5-sunburst` via the
Images API from the project's own prompts (one prompt per state, one style spec per pack), then
cropped, padded and resized; hotspots are derived (topmost opaque pixel for `arrow`/`hand`,
bounding-box centre otherwise). They are part of this repository under its MIT license.
"Classic 2001" and "Aero Glass" are period-*inspired* designs, not copies of any vendor's cursors.
