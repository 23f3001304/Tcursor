# src/editor/motion/presets.ts

The five named feels, as data (M3, `docs/superpowers/specs/2026-09-15-motion-editor-design.md` section 3). Curves are DRAWN, not named, in the new motion editor; presets are the named starting points a drawing begins from, and this file is the whole table.

**TypeScript only, deliberately.** Presets are the only thing that ever WRITES a curve, and only the editor writes curves - Rust just evaluates whatever string arrives, through the same `easing` channel it has always read (`export/spring.rs`, `export/cubic.rs`, and `export/keys.rs` for the new `keys(...)` form). Putting the table here means adding a preset touches one file and no Rust at all.

## MotionPreset

```ts
export interface MotionPreset {
  id: "snappy" | "soft" | "cinematic" | "mechanical" | "bouncy";
  name: string;
  easing: string;
  easing_out: string;
  feel: string;
}
```

One named feel: a pair of curve strings plus its labels.

- `id` - the stable key stored in `MotionSettings.preset` and used by `presetPatch`. A closed union, so a typo in a call site is a compile error rather than a silent fall through to Soft.
- `name` - what the picker shows ("Snappy", "Soft", ...).
- `easing` - the ramp INTO a region (zoom-in, a layout's entry fade, a camera move's blend).
- `easing_out` - the ramp OUT of one (zoom-out, a layout's exit fade). Equal to `easing` for every preset except Cinematic, which is the only asymmetric feel in the table.
- `feel` - the one-line description under the picker, and the option's hover title.

## PRESETS

```ts
export const PRESETS: MotionPreset[]
```

The five, in the order the picker shows them: Snappy, Soft, Cinematic, Mechanical, Bouncy. The strings are the spec's table verbatim, and they are CANONICAL as written: `presetOf` compares against them literally, and Rust's `valid_easing` re-emits exactly these forms, so a pair that round-trips through the doc names the same preset it started as.

**Soft is the bare word `"smooth"`, not the `keys(...)` curve the spec's table gives.** The spec itself says Soft matches `"smooth"`, and writing the word rather than a curve that agrees with it to within 1e-3 buys two things: every region in every project ever recorded already carries `"smooth"`, so they all read back as Soft rather than Custom (and "Apply to all regions" stays an optional sweep instead of a mandatory migration); and the shipped camera trajectory stays bit-identical, which the jank probe's fingerprint (`0x58da_34b7_41e9_0fd3`) pins. `MotionSettings::default()` on the Rust side mirrors this exactly.

Four of the five are `keys(...)` strings, which only parse once the evaluator lands (`src/editor/motion/keys.ts` and `export/keys.rs`, other agents' work). That is fine and intended: nothing in this file parses a curve - it carries strings - and Soft, the default, needs no parser at all.

### Used by

- `src/editor/shell/settings/MotionSection.tsx` - the preset row
- `src/editor/motion/PresetRow.tsx` and the three inspectors (the graph agent's work) - the same table, per region

## presetOf

```ts
export function presetOf(easing: string, easingOut: string | null | undefined): MotionPreset["id"] | "custom"
```

Which preset a curve PAIR is, or `"custom"` when it matches none of the five.

### Inputs

- `easing` - the in ramp's string.
- `easingOut` - the out ramp's. **A null, undefined or empty value means "the same curve as `easing`"** - the wire meaning of an absent `Zoom.easing_out` - so a zoom that stores only one curve still names a symmetric preset. This is the second half of why Soft is `"smooth"`: `presetOf("smooth", undefined)` is `"soft"`, which is every region in every pre-M3 project.

### Returns

The matching preset's `id`, else `"custom"`.

### Implementation

Both strings are run through a small `canonical` helper first, which rewrites a `spring(k,c[,m])` into the 3-decimal form Rust's `export::spring::format_spring` writes (mass defaulting to 1) and leaves everything else trimmed but untouched. *Why:* a Bouncy pair written by this file reads `spring(140,7,1)`, but the same pair read back from a doc that has been through `valid_easing` reads `spring(140.000,7.000,1.000)` - without canonicalisation the picker would flip to Custom the first time the project round-tripped. The `keys(...)` forms need no such handling: `valid_easing` re-emits them in the same canonical form the table already uses.

Both halves must match: Snappy's in ramp paired with Cinematic's out ramp is Custom, and Cinematic's in ramp with NO out ramp is Custom too (Cinematic is the split preset, so "same as easing" is not it).

### Behaviors

- `presetOf` names every preset from its own pair, and round-trips `presetPatch`'s output (`presets.test.ts`).
- `reads a pre-M3 region ... as Soft, never Custom` - `undefined`, `null`, `""` and `"smooth"` all resolve as the out ramp.
- `names Bouncy from the canonical spring Rust writes back, as well as the short literal` - including `spring(140,7)` with the mass omitted.

## presetPatch

```ts
export function presetPatch(id: string): { easing: string; easing_out: string }
```

The `{ easing, easing_out }` patch that picking a preset writes - into `settings.motion` for the project default, or into an `update_zoom` / `update_layout_seg` / `update_camera_move` op for one region.

Takes a plain `string` rather than `MotionPreset["id"]` because its callers pass a value that came off the wire (`settings.motion.preset`, a picker value), and falls back to Soft for anything that is not a preset id. That cannot happen through the picker; it keeps a stale or hand-edited `preset` name from ever producing an `undefined` curve at an add op.
