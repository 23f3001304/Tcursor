# src-tauri/build.rs

The Cargo build script. Runs `tauri_build::build()` (icons, the Windows resource, the capability/permission codegen) and, before it, generates the bundled wallpaper table.

## main

```rust
fn main()
```

Calls `emit_wallpaper_table` then `tauri_build::build()`. *Why the wallpaper scan comes first:* it is a plain file-system pass with no dependency on Tauri's own codegen, and running it first means a bad asset folder fails immediately rather than after the slower Tauri step.

## emit_wallpaper_table

```rust
fn emit_wallpaper_table()
```

Scans `assets/backgrounds/wallpapers/*.jpg` and writes `$OUT_DIR/wallpapers_gen.rs`, which `settings/wallpapers.rs` `include!`s and re-exports as `WALLPAPERS`.

*Why generated rather than a hand-written table:* the wallpaper sets grow (Ribbons, then Folds, then Gradients, Metal, Scenic, with more coming) and land one at a time. A hand-listed table is one more place to forget an id, mistype a name, or leave an orphan `include_bytes!` behind. Here, dropping a JPEG into the folder is the entire change - the id, the display name, the group and the ordering all follow from its filename.

### Rules

Everything is derived from the file stem:

- **id** - the stem itself (`folds-silk-blue`). This is what `BackgroundSettings.mesh` persists, so it must stay stable: renaming a file changes what saved projects point at, and they fall back to the legacy mesh.
- **group** - from the prefix: `folds-` -> `"Folds"`, `gradient-` -> `"Gradients"`, `metal-` -> `"Metal"`, `scenic-` -> `"Scenic"`. Anything with no KNOWN prefix is `"Ribbons"` - the original set, which predates the prefixes and must keep its bare ids. A group whose files have not landed yet simply contributes nothing; the scan never needs to know which sets exist.
- **name** - the stem minus that prefix, hyphens turned into spaces, each word capitalized: `scenic-coast-dusk` -> `Coast Dusk`, `folds-silk-blue` -> `Silk Blue`.
- **order** - Ribbons, Folds, Gradients, Metal, Scenic; alphabetical by display name inside each group. `read_dir` order is arbitrary, so this sort is what makes the generated file stable across machines and rebuilds - and it is also the order the editor's picker renders, one section per group.

### Implementation

1. Emit `cargo:rerun-if-changed=assets/backgrounds/wallpapers` - watching the DIRECTORY, so adding or removing a file re-runs the scan, not just editing one.
2. Collect every `.jpg`, deriving `(group order, name, group, id)` per file, and sort that tuple.
3. Write a `const WALLPAPERS_GEN: &[Wallpaper]` whose `bytes` are `include_bytes!` of an ABSOLUTE path built from `CARGO_MANIFEST_DIR`. *Why absolute:* `include_bytes!` resolves relative to the file it appears in, which for generated code is somewhere under `OUT_DIR`, nowhere near the assets.
4. *Why a private `const` and not the public `static` itself:* the public `WALLPAPERS` (and its doc comment) then lives in a real source file, where the `docs-hover` drift guard can see it - a symbol that exists only inside `OUT_DIR` is invisible to both the validator and the editor hover.

A missing or unreadable asset folder panics the build. That is deliberate: the wallpapers are compiled into the binary, so a silent empty table would ship a product with no wallpapers and no error anywhere.

## title_case

```rust
fn title_case(stem: &str) -> String
```

`coast-dusk` -> `Coast Dusk`. Splits on hyphens, capitalizes each word's first character and leaves the rest alone (so an already-capitalized word is not lowercased).
