# Contributing to TCursor

TCursor is a Windows screen recorder and editor built with Tauri v2: a Rust core in `src-tauri/`, a React and TypeScript front end in `src/`. This page is how work gets into `main`. It is short on purpose, and CI enforces the parts that can be checked by a machine.

## Working principles

1. **Small, single-purpose changes.** One concern per pull request. A change that needs "and" in its title is two pull requests.
2. **The export is the reference.** The exported video is the product. The preview must match it frame for frame, and when the two disagree the fix goes into the preview, never into the export.
3. **One responsibility per file.** Files are budgeted by kind (below). A file that outgrows its budget is split by responsibility into real modules with their own docs pages and tests. Lines are never compressed to fit a number.
4. **The documentation is `docs/api/`, not comments.** Source files carry no narrative comments. The only comment allowed in `.rs`, `.ts` and `.tsx` is a block that starts `// INVARIANT:` and states a rule the code depends on. Everything else a reader needs lives in the page that mirrors the file: `docs/api/<path without extension>.md`.
5. **Tests first.** New behaviour arrives with a test that failed before the change. Numbers that the Rust and TypeScript sides must agree on are pinned on both sides with the same table.
6. **Core logic lives in Rust.** TypeScript mirrors it for the live preview and says so; it does not invent behaviour of its own.
7. **Nothing clever in the interface.** Plain dropdowns and the shared controls over bespoke widgets, colours from CSS tokens only, the `motion` library for stateful animation and no CSS keyframes.
8. **No new dependency without a reason** written in the pull request.

## How to open a pull request

1. Branch from `main`: `feat/<topic>`, `fix/<topic>`, `docs/<topic>` or `chore/<topic>`.
2. Make the change with its tests and its docs pages (next section).
3. Run the gate locally (below) until it is green.
4. Open the pull request against `main` and fill in the template: what and why, how it was verified, the checklist.
5. CI runs the same gate plus the two pull request rules (size and docs pages). A red check is fixed in the same pull request with new commits; do not force-push over a review.
6. A maintainer reviews and merges. `main` always builds and always passes the gate.

Commit messages use a conventional prefix and then say what changed and why in plain sentences, for example `fix(preview): the paused frame is asked for by output time, because after a reorder one source instant can be shown twice`. No trailers.

## Docs pages travel with the code

Every source file under `src/` and `src-tauri/src/` has a page at `docs/api/<same path, extension dropped>.md` (`src-tauri/src/export/remap.rs` is documented in `docs/api/src-tauri/src/export/remap.md`), with one `## symbol` section per public item. The editor extension in `tools/docs-hover` shows these pages on hover, which is why they must stay true.

- A changed source file changes its page in the same pull request. A new file adds its page. A removed symbol loses its section.
- `npm --prefix tools/docs-hover run validate` must print `docs-hover: all doc sections resolve to symbols`. It fails on a section whose symbol no longer exists.
- Some pages cite design notes under `docs/superpowers/`. Those are the maintainer's working notes and are not part of this repository; the page itself is the contract.
- CI fails a pull request in which a source file changed and its page did not. For the rare change that truly cannot affect the page (a rename of a private local, a formatting-only change), a maintainer adds the `docs-not-needed` label.

## Pull request size

A pull request changes **at most 400 lines of source** (added plus deleted). Tests, docs pages, fixtures, lockfiles and assets do not count, so there is never a reason to skip a test or a docs page to fit. `node tools/ci/pr-rules.mjs origin/main` prints your current count.

Larger work is delivered as a stack of pull requests that each build and pass on their own: first the model or the pure function with its tests, then the code that consumes it, then the interface. When a single change really cannot be split (a mechanical rename, a generated table), a maintainer adds the `large-pr` label.

## The gate

From `src-tauri/`:

```bash
cargo fmt --check
cargo build --tests
cargo test
```

From the repository root:

```bash
npx tsc --noEmit
npx vitest run
npx prettier --check "src/**/*.{ts,tsx}"
node tools/linecount.mjs --check
npm --prefix tools/docs-hover run validate
```

Building the Rust crate needs the prerequisites listed in the README (CMake, the MSVC C++ workload, LLVM for `libclang`, the Vulkan SDK) and the two FFmpeg executables, which `pwsh tools/ci/fetch-ffmpeg.ps1` downloads and verifies. Five integration tests need real hardware or a real recording and are `#[ignore]`d; the README lists how to run them by hand.

## Line budgets

| Kind | Budget |
|---|---|
| Rust logic (`*.rs`) | 280 lines |
| Rust test modules (`*_tests.rs`) | 320 lines |
| Rust integration tests (`src-tauri/tests/`) | 280 lines |
| TypeScript (`*.ts`, `*.tsx`) | 200 lines |
| TypeScript tests (`*.test.ts`, `*.test.tsx`) | 220 lines |

`node tools/linecount.mjs --all` lists every file with its count. Declaration-only files (one long enum or table) are listed as exempt inside the tool.

## Things that never change in passing

Some tests pin numbers that existing projects depend on: the camera's jank fingerprint (`jank_probe_tests`), the byte-identical round trip of three saved documents (`roundtrip_tests`), and the time-map parity tables (`remap_tests.rs`, `remap_clips_tests.rs`, `remap.test.ts`, `remapClips.test.ts`). A change that needs one of those numbers to move is a change in its own pull request that says so in its title.

## Releases and code signing

A maintainer tags `v<version>` on `main`. The Release workflow builds the installers from that exact commit on a clean GitHub runner and opens a draft release with their SHA-256 sums; a maintainer reviews the draft and publishes it. Nobody uploads a locally built binary. The code signing policy is in the README.

## Reporting a problem

Open an issue with what you did, what you expected and what happened, the TCursor version, and if it is about a recording or an export, whether it reproduces on a fresh short take. Security problems go to the maintainer privately through GitHub's "Report a vulnerability".
