# TCursor

An auto-zoom screen recorder for Windows that turns a raw capture into a polished, intentional-looking
demo — automatic cursor-follow zooms, a built-in editor, and a local-AI "director" that suggests edits.

Built with Tauri v2 (Rust) and React/TypeScript. GPU compositor (wgpu) for export, FFmpeg for
muxing and proxies.

## Install

Download `TCursorSetup.exe` from the [Releases](../../releases) page and run it.

> The installer isn't code-signed yet, so Windows SmartScreen will show "Windows protected your PC".
> Click **More info → Run anyway**. See [signing](src-tauri/installer/SIGNING.md) for the plan to
> fix this (free for open source via SignPath Foundation). You can also build from source, which
> never triggers SmartScreen.

## Build from source

Requires Node 20+, Rust (stable), and `src-tauri/resources/ffmpeg.exe` + `ffprobe.exe`.

```bash
npm install
npm run tauri dev      # run the app
npm run tauri build    # build app + NSIS/MSI installers
```

To produce the branded single-file installer (`setup/dist/TCursorSetup.exe`):

```bash
pwsh ./setup/build.ps1
```

## Docs-hover extension (developer IntelliSense)

Function documentation lives in `docs/api/` (mirroring the source tree), not in code — this keeps
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

- `src/` — React/TS frontend (recorder HUD + editor)
- `src-tauri/` — Rust backend (capture, export pipeline, GPU compositor, AI)
- `setup/` — branded installer app ("TCursor Setup") over the silent NSIS installer
- `docs/api/` — the hover-docs store; `docs/superpowers/specs/` — design specs
- `tools/docs-hover/` — the VS Code hover extension

## License

MIT — see [LICENSE](LICENSE). You're free to use, modify, and redistribute it, but the copyright
notice must be kept, so the original author is always credited.

Created by Coehe.
