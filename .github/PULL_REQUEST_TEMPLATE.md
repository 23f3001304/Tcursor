## What and why

<!-- One concern per pull request. Say what changes for the user or the codebase, and why. -->

## How it was verified

<!-- The commands you ran and what they printed (test counts). For anything visual: what you looked at. -->

## Checklist

- [ ] One concern, and at most 400 changed lines of source (tests, docs pages, lockfiles and assets do not count). Bigger work is split into a stack of pull requests.
- [ ] Every changed source file brings its `docs/api/` page along in this pull request; a new file adds its page, a removed symbol loses its section.
- [ ] `npm --prefix tools/docs-hover run validate` prints `docs-hover: all doc sections resolve to symbols`.
- [ ] Tests first: new behaviour has a test that failed before the change.
- [ ] The gate is green locally: `cargo fmt --check`, `cargo build --tests`, `cargo test` (from `src-tauri/`), `npx tsc --noEmit`, `npx vitest run`, `npx prettier --check "src/**/*.{ts,tsx}"`, `node tools/linecount.mjs --check`.
- [ ] No file over its line budget, and nothing compressed to fit one.
- [ ] Export and preview still agree (if this touches either one, the fix went into the preview).
- [ ] No new dependency, or the pull request says why it is needed.
