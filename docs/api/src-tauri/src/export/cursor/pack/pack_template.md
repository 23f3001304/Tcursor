# src-tauri/src/export/cursor/pack/pack_template.rs

M8(c): "Create pack template". Writes a fresh, complete pack folder so a would-be pack author never has to read `pack.rs` to learn the on-disk contract. Sprites are copied from the bundled "Clean" pack (`macos-clean` - its busy sprite is a spinner, meant to animate), falling back to the embedded default set's own compiled-in bytes for any state Clean doesn't resolve, so the template always ships all nine states and renders immediately. The written `pack.json` is v2 with a declared busy animation. `README.txt` (plain ASCII, `include_str!`'d from `pack_template_readme.txt` beside this file) states the whole contract in prose - file names per state, the hotspot rule, every `pack.json` field, the busy animation options, explicit `busy_NN.png` frames, the `material` field, and where an imported pack ends up.

## create_pack_template

```rust
#[tauri::command]
pub fn create_pack_template(dir: String) -> Result<String, String>
```

Write a new template pack folder directly under `dir` (a folder the frontend's folder-picker dialog returned), named `my-pack`, or `my-pack-2`, `my-pack-3`, ... if that name is already taken there. Returns the created folder's absolute path.

### Inputs

- `dir: String` - absolute path to an existing folder the user picked. *Why a fresh folder each call rather than always `my-pack`:* running the command twice against the same parent must never clobber an in-progress edit of the first template.

### Returns

`Ok(String)` - the new folder's absolute path, on success. `Err(String)` when `dir` is not a directory, or when the write itself fails (disk full, permissions) - in which case any partial folder is best-effort removed before returning the error, so a retry doesn't see a half-written template.

### Implementation

1. Reject a `dir` that isn't a directory.
2. `unique_template_dir(&root)` - `my-pack`, or the first untaken `my-pack-N`.
3. `write_template` - on failure, best-effort `remove_dir_all` the partial folder.
4. Return the new folder's path as a string.

### Used by

- `src-tauri/src/lib.rs` - registered in `invoke_handler!`
- `src/shared/ipc.ts` (`createPackTemplate`) - the TS wrapper
- `src/editor/panels/cursor/CursorPackField.tsx` - calls after the user picks a folder via the Tauri dialog plugin, then reveals the result with the opener plugin

## unique_template_dir

```rust
fn unique_template_dir(root: &Path) -> PathBuf
```

`root/my-pack`, or the first of `root/my-pack-2`, `root/my-pack-3`, ... that doesn't exist yet.

### Behaviors

- `unique_template_dir_suffixes_on_collision` - an empty `root` yields `my-pack`; once that exists, `my-pack-2`; once that also exists, `my-pack-3`.

## write_template

```rust
fn write_template(dir: &Path) -> std::io::Result<()>
```

Write every file a valid pack folder needs directly at `dir`: the nine sprite PNGs, `hotspots.json`, `pack.json`, and `README.txt`. Takes `dir` directly (rather than deriving it from a picked parent folder internally) so it is unit-testable against a temp folder.

### Implementation

1. `create_dir_all(dir)`.
2. For each `(kind, builtin_png, _)` in `cursorset::SPRITES`: try reading `{kind_filename(kind)}` from the bundled Clean pack's folder (`packdirs::bundled_pack_dir("macos-clean")`), treating a missing or empty file as absent; fall back to `builtin_png` (the embedded default set's own compiled-in bytes) when Clean doesn't resolve at all or doesn't have that file. Write whichever bytes won under `dir/{kind_filename(kind)}`.
3. Write the constant `HOTSPOTS_JSON`, `PACK_JSON`, and `README_TXT` (see below) verbatim.

### Behaviors

- `create_pack_template_writes_a_folder_the_real_loader_reads_back` - runs the actual command against a temp `root`, then re-resolves the written folder through `pack::sprite_sources_from_dir` (the same function the export and preview use): all nine `CursorType`s come back, and each row's bytes match the file on disk - i.e. every state was genuinely read from what this function wrote, not silently falling through to a builtin default. Also parses `pack.json` through `pack::read_meta` (`id`, `name`, and a `Some` `busy` spec), parses `hotspots.json` as `{kind: [hx, hy]}` covering all nine states, and checks `README.txt` mentions `pack.json`, `hotspots.json` and `busy` and is plain ASCII.
- `creating_a_second_template_in_the_same_folder_does_not_clobber_the_first` - two calls against the same `root` return two different, both-existing folders.

## TEMPLATE_SOURCE_ID

```rust
const TEMPLATE_SOURCE_ID: &str = "macos-clean";
```

The bundled pack id a fresh template's sprites are seeded from. *Why Clean and not the embedded default set:* the embedded set's `busy.png` is a static multicolor pinwheel disc that `pack::busy_as_arrow` exists specifically to hide (see `pack.md`) - it would make a poor advertisement for the very animation the template's `pack.json` declares. Clean's busy sprite is a plain spinner, drawn to actually spin.

## HOTSPOTS_JSON

```rust
const HOTSPOTS_JSON: &str
```

One hotspot per state, as canvas fractions - the tip for `arrow`/`hand`, the centre for every other state, matching the rule the bundled packs themselves were generated by (`assets/cursorpacks/README.md`). Written verbatim as `hotspots.json`, independent of whichever sprite bytes (Clean's or the builtin fallback) ended up on disk for a given state - the values are close enough to correct for either source that a template author edits them anyway once they draw their own art.

## PACK_JSON

```rust
const PACK_JSON: &str
```

A complete v2 manifest: `id`, `name`, `category` (`"Imported"`, matching what a real import would default to) for the picker, `version: 2`, and a declared `busy: { anim: "spin", fps: 24 }` so the template demonstrates the format's headline feature rather than shipping a static disc. `material` is deliberately left out: it is documented in the README as opt-in, and the copied artwork is a plain picture, not a glass lens (see `pack.md`'s `Meta::material`) - declaring `"glass"` over ungainly opaque sprites would misrepresent what the field is for.

## README_TXT

```rust
const README_TXT: &str = include_str!("pack_template_readme.txt");
```

The folder contract in prose, `include_str!`'d from the sibling `pack_template_readme.txt` so the (fairly long) text is edited as prose rather than as a Rust string literal. Plain ASCII, no em dashes - pinned by `create_pack_template_writes_a_folder_the_real_loader_reads_back`'s `is_ascii()` check. Covers, in order: the nine sprite filenames and the "missing state falls back to the builtin" rule; `hotspots.json`'s shape and its centered default; every `pack.json` field including `material`; the three `busy.anim` values and what each looks like; explicit `busy_NN.png` frames and that they take precedence over a declared `anim`; and where a pack ends up once "Import pack..." picks this folder.
