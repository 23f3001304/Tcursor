# src-tauri/src/asr/mod.rs

MODULE OVERVIEW: everything speech. The captions milestone puts on-device transcription behind one module so that nothing outside it knows what a Whisper model is, where its file lives, or how it got there.

The floor is a table of the models the app knows about, a resumable and sha256-checked download into the app data dir, and the IPC the Captions panel drives it with. Transcription stands on it, split so that all but ONE file is pure and runs with no model on disk: `audio` picks and decodes the recording's own WAV and carries the output-clock shift, `words` turns whisper's sub-word tokens into words, `group` turns words into caption lines, and `whisper` is the only file in the tree that knows the crate exists. Every consumer reads the model through `models::resolve_model` rather than by path.

*Why a model store at all, rather than bundling a model with the installer:* `base.en` alone is 141 MiB and `small.en` is 465 MiB, which is several times the size of the whole app. Shipping one would make every download pay for a feature most takes never use, and shipping all four is out of the question. The cost of downloading on first use is one progress bar, once, on a machine the user is already sitting at.

## audio

Picking the track to transcribe, decoding it to 16 kHz mono f32 through the bundled ffmpeg, and the pure arithmetic that moves a word from the WAV's clock onto the doc's output clock. See `asr/audio.md`.

## commands

The Tauri IPC surface: `whisper_models` (the table plus what is on disk), `download_whisper_model` (fetch one, on a thread, reporting through events) and `transcribe_project` (the whole pass, writing the caption track into the doc itself). See `asr/commands.md`.

## group

Words into caption lines - the size, silence and sentence rules, all pure. See `asr/group.md`.

## download

Fetching a model file resumably, and refusing to install one whose digest is wrong. See `asr/download.md`.

## models

The model table, the paths its files live at, and `resolve_model` - the one place that decides which model a `CaptionStyle` actually means. See `asr/models.md`.

## sha256

A hand-rolled FIPS 180-4 SHA-256, so checking a 465 MiB download costs no dependency. See `asr/sha256.md`.

## whisper

The whisper.cpp run, and the only file that knows `whisper-rs` exists. Returns RAW tokens, which is the seam that keeps everything else testable. See `asr/whisper.md`.

## words

Whisper's sub-word tokens into words, by the leading space that marks a word boundary. Pure. See `asr/words.md`.
