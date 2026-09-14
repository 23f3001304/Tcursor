# Bundled cursor packs

One folder per pack, in the same shape an imported pack uses (`export/cursor/pack.rs`): nine
sprites named by `CursorType` wire name (`arrow`, `ibeam`, `hand`, `resize_ns`, `resize_ew`,
`resize_nwse`, `resize_nesw`, `move`, `busy`), `hotspots.json` (`{kind: [hx, hy]}` as canvas
fractions) and `pack.json`. Sprites are 128x128 RGBA PNGs with straight alpha.

`pack.json` `category` is the STYLE section the editor's pack picker lists the pack under, and the
only thing that decides where it appears: `Classic` (plain system arrows), `Glass and glow`,
`Playful`, `Drawn`, `Retro`. A pack that names none - which is every pack a user imports, since
`pack_import` writes a v1 manifest - lists under `Imported`, and a category outside that list gets
a section of its own after the curated ones. The embedded "default" set has no manifest at all, so
its category is stated in `packlist::DEFAULT_PACK_CATEGORY` instead. Adding a pack to a section is
therefore a one-line edit here, never a change in the frontend.

`pack.json` `material` says how the renderer TREATS the sprites, as opposed to what they depict.
Absent (every pack but one) is a plain alpha blit. `"glass"` makes each sprite a LENS: the FX pass
refracts the recorded frame through the sprite's own alpha - a radial displacement toward its
centre, a four-tap frost, a brightness lift and a cool cast, with a soft drop shadow underneath -
and the sprite's pixels are then blitted at 65% so its baked highlights and rim sit ON the live
refraction instead of hiding it. Artwork for a glass pack should therefore be a CLEAR lens with
highlights and a rim, not a picture of glass: whatever it paints opaquely, the frame cannot bend
through. See `export/fx/fx_lens.rs` and `fx_lens.wgsl`. The glass DISC behind the cursor is a
separate, pack-independent setting (`CursorSettings.back`), not a manifest key.

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
