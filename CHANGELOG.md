# Changelog

What changed in each TCursor release, newest first. The Release workflow copies the section of the
tagged version into the GitHub release (`tools/ci/release-notes.mjs`) and refuses to build a release
whose version has no section here, so this file is where "what changed" gets written: in the same
pull request as the change, or at the latest before tagging. A release candidate (`v0.1.0-rc.1`)
uses the section of the version it is a candidate for.

## 0.1.0

The first public release, so everything below is new.

### Recording

- Records a display together with the webcam, the microphone and the system audio, each of which can be switched off. Pause and resume are exact: paused time is removed from the video, the audio and the recorded input alike.
- The camera, the microphone and the recorded display can be switched in the middle of a take. The segments are joined when the take stops.
- Frames are encoded on the GPU through Media Foundation, with FFmpeg as the compatibility encoder. Every frame keeps its own capture time, so the export stays in sync with the audio even when the capture rate varies.
- While a take runs the HUD shrinks to one small pill (webcam, timer, level meter, Pause, Stop) that is kept out of the capture.

### Auto-zoom and cursor

- The camera zooms in on clicks and typing by itself and follows the cursor while zoomed, and a hotkey holds a manual zoom. Every automatic zoom becomes an editable region on the timeline.
- The cursor is redrawn rather than recorded: smoothed along its path, tilted slightly in the direction it moves, and drawn from one of 16 bundled cursor packs or a pack you import.
- Click effects, and a cursor spotlight with six looks: classic, blur, halo, breathing, nebula and vignette.

### Editor

- A timeline with lanes for zooms, effects, layouts, camera moves, text, captions, clips and audio, an inspector for whatever is selected, and undo and redo for every edit.
- Trim, cuts and speed changes share one time map, so the picture, the audio and every effect stay aligned. "Remove silences" cuts the quiet stretches in one step.
- Clips: split at the playhead (B), reorder, trim an edge, and dissolve from one clip into the next.
- Layouts arrange the screen and the webcam. Panels can be dragged on the stage with smart guides, and the webcam can move, resize and change shape over time with keyframes.
- Masks that blur, pixelate or highlight a rectangle, a colour grade picked from a table of presets, and animated text in four kinds: title, lower third, big stat and callout.
- Backgrounds from the bundled wallpapers and gradients or from your own image, and the usual aspect ratios, vertical included.
- A paused preview frame is rendered by the export's own renderer, so what you line up in the editor is what the export draws.

### Captions and AI director

- Captions are transcribed on your machine by whisper.cpp, on the GPU through Vulkan (NVIDIA, AMD and Intel from the same build). The speech model is downloaded the first time you ask for captions. Captions can be edited and are drawn into the export.
- The AI director looks at frames of the recording with a local model served by Ollama and proposes an edit plan, which you review before anything is applied.

### Export and projects

- A GPU compositor renders the export and a hardware encoder writes it where there is one, with a dialog for format, resolution, frame rate and quality.
- A recording is a folder with a `.tcursor` project file that opens in the editor on a double click.

### Install and privacy

- Three installers, built from the tagged commit on a clean GitHub runner and listed with their SHA-256 checksums: the branded `TCursorSetup.exe`, a plain NSIS installer and an MSI. They are not code-signed yet.
- No account, no telemetry and no update check. The only network use is the speech model download and the local Ollama server, both started by you.
