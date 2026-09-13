# setup/src-tauri/build.rs

The Setup app's build script. Bakes in the three things the window cannot work out at run time:
the NSIS installer to run, how many bytes it will write, and the license to show.

Everything it produces lands in `OUT_DIR` and is pulled in by `include_bytes!` / `include_str!` /
`include!`, so a plain `cargo build` in `setup/src-tauri` is a complete build with no extra step -
it just produces an app that knows it has no payload.

## main

```rust
fn main()
```

In order: stage the payload, write `expected_bytes.rs`, write `eula.txt`, then `tauri_build::build()`.

**The payload.** `setup/build.ps1` copies the main app's NSIS installer to
`setup/src-tauri/assets/nsis-setup.exe` (gitignored, ~58MB). If it is there and larger than 1KB it
is copied into `OUT_DIR`; otherwise an empty placeholder is written and a `cargo:warning` is
printed. The placeholder keeps `cargo build`, `cargo check` and `cargo test` usable during
development without a 58MB file in the tree, and `install::stage` refuses to run anything that
small, so a dev build cannot silently launch nothing.

**The two `cargo:rerun-if` lines at the end** are what make a re-staged payload or a changed
`TCURSOR_INSTALL_BYTES` re-run this script; `expected_bytes` adds a third for the NSIS script it
reads.

## expected_bytes

```rust
fn expected_bytes(manifest: &Path) -> u64
```

The uncompressed install size, or 0 if this build cannot know it.

*The problem.* NSIS run with `/S` reports nothing, so the window measures the destination folder
instead and needs a denominator. The payload we embed is a copy, and nothing about that copy says
how big the thing it installs is.

*How it is derived.* `CARGO_MANIFEST_DIR` is `<repo>/setup/src-tauri`, so two `parent()` calls give
the repo root, and the main app's bundle staging folder is
`<repo>/src-tauri/target/release/nsis/x64/`. That folder holds the `installer.nsi` the bundler
generated for this very payload, and near the top of it:

```
!define ESTIMATEDSIZE "224113"
```

That is the bundler's own sum of every file the installer places in `$INSTDIR`, in KB (it is what
NSIS writes to the `EstimatedSize` value in Add/Remove Programs). Times 1024 it is the number the
progress bar wants, from the same tool that packed the payload, with no guessing about which files
are in the bundle.

*Why not sum a directory.* There is no directory that is the payload. The staging folder holds only
the script and its includes; the bundler's `File` directives point at absolute paths scattered
across `target/release` (the main binary) and `src-tauri` (resources), next to megabytes of build
artifacts that are not installed. Summing any enclosing folder would be wrong by an order of
magnitude.

*The fallback.* If the staging folder is not there - a tree where it was pruned, or a `cargo build`
run outside the repo - the `TCURSOR_INSTALL_BYTES` environment variable is read instead.
`setup/build.ps1` sets it from the same `ESTIMATEDSIZE`, so a shipping build has two independent
paths to the same number. If both fail the constant is 0 and `progress::step` falls back to timed
pacing, which `main` warns about at build time.

## estimated_size_kb

```rust
fn estimated_size_kb(nsi: &str) -> Option<u64>
```

Pulls the number out of the `!define ESTIMATEDSIZE "N"` line. Matches on the line prefix and takes
the first quoted field, so it cannot be fooled by the word appearing in a comment or by whitespace
changes in the generated script.

## license_text

```rust
fn license_text(manifest: &Path) -> String
```

Reads `<repo>/src-tauri/installer/eula.rtf` - the same file `bundle.licenseFile` points the NSIS
wizard at - and flattens it. A missing file yields a one-line placeholder rather than a build
failure, because the Setup app is buildable outside the repo and a missing license is not a reason
to have no installer.

## rtf_to_text

```rust
fn rtf_to_text(rtf: &str) -> String
```

A scanner, not an RTF implementation, covering the subset `eula.rtf` uses: `\par` and `\line` become
newlines, `\tab` a tab, `\endash` and `\emdash` a hyphen, `\\` `\{` `\}` their literal characters,
the ignorable destinations (`\fonttbl`, `\colortbl`, `\stylesheet`, `\info`) are skipped to their
closing brace, every other control word is dropped, and braces and raw newlines carry no text.

*Why this belongs at build time:* the result is constant for a given build, so converting it once
here costs nothing at run time and keeps the parser out of the 200-line budget of a source file.
Anything richer than this subset (tables, images, embedded fonts) would need a real parser, and the
answer then is to keep the plain text in the repo rather than to grow this function.
