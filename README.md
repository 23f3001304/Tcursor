# TCursor

An auto-zoom screen recorder for Windows that turns a raw capture into a polished, intentional-looking
demo - automatic cursor-follow zooms, a built-in editor, and a local-AI "director" that suggests edits.

Built with Tauri v2 (Rust) and React/TypeScript. GPU compositor (wgpu) for export, FFmpeg for
muxing and proxies.

## Install

Download `TCursorSetup.exe` from the [Releases](../../releases) page and run it. Every release is
built from its tagged commit by the [Release workflow](.github/workflows/release.yml) on a clean
GitHub runner, and ships with a `SHA256SUMS.txt` you can check the download against. What changed in
each version is in [CHANGELOG.md](CHANGELOG.md), and every release's description repeats its section.

> The installers are not code-signed yet, so Windows SmartScreen shows "Windows protected your PC".
> Click **More info**, then **Run anyway**. TCursor uses the SignPath Foundation for code signing
> (see [Code signing policy](#code-signing-policy)); until the application is approved the releases
> stay unsigned. You can also build from source, which never triggers SmartScreen.

## Build from source

Requires Node 24 (the version CI runs), Rust (stable), `src-tauri/resources/ffmpeg.exe` + `ffprobe.exe` (`pwsh tools/ci/fetch-ffmpeg.ps1` downloads the pinned build and checks its SHA-256), and for the on-device captions (whisper.cpp with its Vulkan GPU backend): CMake 3.14+ on PATH, the MSVC C++ workload, an LLVM whose `libclang` bindgen can find (`LIBCLANG_PATH`), and the Vulkan SDK (`winget install KhronosGroup.VulkanSDK`). The Rust crate builds into `C:/tct` (`src-tauri/.cargo/config.toml`) because whisper's shader generator overruns Windows' path limit from a deep checkout. GPU inference works on NVIDIA, AMD and Intel from the same build; an NVIDIA-only CUDA backend is the opt-in cargo feature `asr-cuda` (needs the CUDA toolkit).

```bash
npm install
npm run tauri dev      # run the app
npm run tauri build    # build app + NSIS/MSI installers
```

To produce the branded single-file installer (`setup/dist/TCursorSetup.exe`):

```bash
pwsh ./setup/build.ps1
```

## Line budgets

Files are budgeted by kind, not by one flat number (the old 200-line cap was retired on 2026-09-15 once `cargo fmt` replaced the one-line golf it had produced). Rust logic 280 lines, Rust test modules 320, Rust integration tests 280, TypeScript 200, TypeScript tests 220; a declaration-only file (one long enum or table, or the HUD icon set) is exempt. Measured on 2026-09-15 after `cargo fmt` and prettier, each number sits at about the 92nd percentile of its kind. One responsibility per file is the real rule: split by responsibility, never merely to fit a number. `node tools/linecount.mjs` lists every file over its budget, and `--check` makes that a failure (CI runs it that way). Formatting is `cargo fmt` with the defaults pinned in `src-tauri/rustfmt.toml` and `npm run format` (prettier, print width 110, CRLF; `.prettierrc.json`) for the TypeScript; the docs live in `docs/api` (see below), so sources carry no doc comments.

## Docs-hover extension (developer IntelliSense)

Function documentation lives in `docs/api/` (mirroring the source tree), not in code - this keeps
files under the project's 200-line budget. A small VS Code extension renders those docs on hover.

```bash
cd tools/docs-hover
npm install
npm run compile
```

Then open `tools/docs-hover` in VS Code and press **F5** for an Extension Development Host, or install
it permanently:

```bash
npx vsce package
code --install-extension tcursor-docs-hover-0.1.2.vsix
```

Keep docs honest with `npm --prefix tools/docs-hover run validate` (fails on any doc section that no
longer resolves to a real symbol). More detail in [tools/docs-hover/README.md](tools/docs-hover/README.md).

## Project layout

- `src/` - React/TS frontend (recorder HUD + editor)
- `src-tauri/` - Rust backend (capture, export pipeline, GPU compositor, AI)
- `setup/` - branded installer app ("TCursor Setup") over the silent NSIS installer
- `docs/api/` - the hover-docs store, one page per source file
- `tools/docs-hover/` - the VS Code hover extension
- `tools/ci/` - the pull request rules and the pinned FFmpeg download that CI uses

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md) has the working principles, how to open a pull request, the rule that docs pages travel with the code, the 400-line pull request limit and the gate CI runs.

## Manual tests

Five integration tests under `src-tauri/tests/` need real hardware or a real recording, so they are `#[ignore]`d and a normal `cargo test` never runs them. Each one skips with a message when its input is absent rather than failing. Run one from `src-tauri`:

| Test | Needs | Run |
|---|---|---|
| `manual_capture` | a display to capture, with the app window on screen over your desktop | `cargo test --test manual_capture -- --ignored --nocapture` |
| `manual_mic` | a microphone as the default input device | `cargo test --test manual_mic -- --ignored --nocapture` |
| `manual_system_audio` | something audible playing on the default output device | `cargo test --test manual_system_audio -- --ignored --nocapture` |
| `manual_mouse` | you, moving and clicking the mouse for a few seconds | `cargo test --test manual_mouse -- --ignored --nocapture` |
| `manual_export` | the newest recording under `%USERPROFILE%\Videos\TCursor` that has `video.mp4` and `events.json` | `cargo test --test manual_export -- --ignored --nocapture` |

They are the only coverage of capture, the mic, loopback and the mouse hook, which cannot run headless.

## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org).

Status: TCursor is applying to the SignPath Foundation programme. Until the application is approved, releases are unsigned; once it is, every release is signed through the Release workflow and nothing else is ever signed.

- Committers and reviewers: [Hemang Choudhary](https://github.com/23f3001304)
- Approvers: [Hemang Choudhary](https://github.com/23f3001304)

Only binaries built by the [Release workflow](.github/workflows/release.yml) from a tagged commit of this repository are submitted for signing, and each signing request is approved by hand.

## Privacy

This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it.

TCursor records, edits and exports on your machine. It has no account, no telemetry and no update check. It uses the network in exactly two cases, both started by you: downloading a speech-recognition model from huggingface.co the first time you ask for captions, and talking to an [Ollama](https://ollama.com) server on your own machine (`localhost:11434`) when you run the AI director.

## License

MIT - see [LICENSE](LICENSE). You're free to use, modify, and redistribute it, but the copyright
notice must be kept, so the original author is always credited.

Created by Coehe.
