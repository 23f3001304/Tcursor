# src-tauri/src/asr/models.rs

The table of Whisper GGML models the app knows about, and where their files live on disk. Nothing here downloads or reads a model: this is the pure lookup layer that `download.rs`, `commands.rs` and the transcriber all agree on, so there is exactly one place that says what `"base.en"` means.

*Why a hard-coded table and not a directory scan:* a scan cannot tell a truncated `ggml-base.en.bin` from a whole one, cannot report a size before the download starts, and cannot say whether a file is multilingual. Pinning the four entries by size and digest means the panel can warn honestly ("465 MB"), the downloader can resume against a known length, and a corrupt file is caught at install time rather than as a crash inside whisper.cpp.

## ModelSpec

```rust
pub struct ModelSpec {
    pub id: &'static str, pub file: &'static str, pub bytes: u64,
    pub sha256: &'static str, pub multilingual: bool, pub label: &'static str,
}
```

One row of the table.

- `id` - the stable identifier stored in `CaptionStyle.model` and sent over IPC (`"base.en"`).
- `file` - the on-disk and on-Hugging-Face filename (`"ggml-base.en.bin"`).
- `bytes` - the exact file size. Used as the progress bar's denominator and as the resume decision's `total`, NOT a Content-Length: a resumed download's bar has to start where it left off, and the server's header describes only the remaining range.
- `sha256` - the file's digest, checked before the file is ever moved into place.
- `multilingual` - whether the model can detect a language, which is what `language: "auto"` requires.
- `label` - the picker's text.

## MODELS

```rust
pub const MODELS: [ModelSpec; 4]
```

The four models the spec names: `tiny.en`, `base.en` (the default), `small.en` and `base` (multilingual). In picker order, smallest English model first.

Sizes and digests were read off the Hugging Face tree API for `ggerganov/whisper.cpp` at `main` (`https://huggingface.co/api/models/ggerganov/whisper.cpp/tree/main`), where each file's `lfs.oid` IS its sha256 and `size` is its exact byte count. They were re-verified against that API on 2026-09-15 when this file was written.

*Why a mismatch is a hard error and never a warning:* a partially-downloaded GGML file does not announce itself. whisper.cpp will happily map it, read a garbage tensor header, and either produce nonsense or abort inside C++ where there is no useful Rust error to show. Refusing the file at install time is the only place the failure can still be explained.

### Behaviors

- `every_entry_has_a_64_hex_digest_a_plausible_size_and_a_unique_id` - the table's own shape: unique ids (a duplicate would make `CaptionStyle.model` ambiguous), 64 lowercase hex digits, a size in the 50 MB to 1 GB band, and a `ggml-*.bin` filename. A typo in a pasted digest is the realistic failure here.
- `the_table_carries_the_four_models_the_spec_names` - the exact byte counts for the three English models and the multilingual flags, so silently swapping a row for a different file breaks a test rather than a download.

## VAD_MODEL

```rust
pub const VAD_MODEL: ModelSpec
```

The Silero voice-activity model whisper.cpp runs before decoding (`ggml-silero-v5.1.2.bin`, 885,098 bytes, sha256 `29940d98...ea2cf`, read off the file itself on 2026-09-15 after downloading it from `ggml-org/whisper-vad`). Not in `MODELS`: it is not a choice the picker offers, it is fetched best-effort by `transcribe_project` before every run and handed to `whisper::AsrParams.vad_model`. Its `multilingual` flag is `true` only because the field has to hold something; a VAD has no language. Added on 2026-09-15 after the first real transcription showed base.en decoding silence into "you".

## vad_path

```rust
pub fn vad_path() -> PathBuf
```

`model_dir()` joined with the VAD file's name, next to the speech models.

## find

```rust
pub fn find(id: &str) -> Option<&'static ModelSpec>
```

Look up a row by id. `None` for anything not in the table - a hand-edited settings file, or a project saved by a newer build. `VAD_MODEL` is deliberately not findable this way, so it can never be picked as a caption model.

## model_dir

```rust
pub fn model_dir() -> PathBuf
```

`<config dir>/TCursor/models/whisper` - the same idiom `settings::store`, `cursor::packdirs` and `project::recents` already use. Tauri's `app_data_dir` is used nowhere in this tree and is deliberately not introduced here. Falls back to the temp dir when the platform has no config dir, which keeps the function total; a download into temp still works, it just does not survive a reboot.

### Behaviors

- `models_live_under_the_apps_own_data_dir` - the path really ends in `models/whisper` under a `TCursor` folder, on either separator.

## model_path

```rust
pub fn model_path(id: &str) -> PathBuf
```

Where the file for `id` would be. Returns a path ending in `unknown.bin` for an unknown id rather than an `Option`, because every caller either already validated the id or is only asking `exists()`.

## is_installed

```rust
pub fn is_installed(id: &str) -> bool
```

Whether a KNOWN id's file is on disk right now. An unknown id is never installed, so the `unknown.bin` path above can never be mistaken for a real model.

This is an `exists()` check, not a digest check: it runs once per row every time the panel opens, and hashing 900 MB to paint a list would be absurd. The digest is checked at download time and again before any file is moved into place, which is where a corrupt file actually matters.

### Behaviors

- `installed_means_a_known_id_whose_file_is_really_on_disk` - an unknown id is never installed; the positive half is gated on the real `ggml-base.en.bin` and prints a SKIP line when it is absent (the plan's ADDED-10 - no test ever downloads).

## url_for

```rust
pub fn url_for(m: &ModelSpec) -> String
```

The download URL: the `ggerganov/whisper.cpp` repository on Hugging Face at `main`, which is where the whisper.cpp project itself publishes these files; `VAD_MODEL` alone comes from `ggml-org/whisper-vad`, the repository whisper.cpp's own README points at for its VAD models.

### Behaviors

- `the_download_url_is_the_pinned_whisper_cpp_repo` - the exact URL for `base.en`, so a change of host is a deliberate edit to a test rather than a silent redirect.

## resolve_model

```rust
pub fn resolve_model(style: &CaptionStyle) -> Result<&'static ModelSpec, String>
```

The model a transcription should actually use, or a message the panel shows verbatim. Two ways to fail, both of which say what to do next:

- an id that is not in the table (`"Unknown caption model ... Pick one from the Captions panel."`);
- `language: "auto"` against an English-only model, which names the multilingual model to switch to.

*Why an error rather than a quiet fallback:* falling back to `base.en` when the user asked for automatic language detection would transcribe a French recording into plausible-looking English nonsense, and the user would have no way to tell that their setting had been ignored. The failure has to be visible before the transcription runs, not discovered in the caption track afterwards.

### Behaviors

- `auto_language_needs_a_multilingual_model_and_says_so` - `auto` + `base.en` is an error whose message names the multilingual model by the LABEL the picker shows (`"Base (multilingual)"`, read out of the table rather than spelled twice); `auto` + `base` resolves; the default style resolves to `base.en`. It asserts against the label rather than the id because the label is the only name the user can act on - the id never appears in the UI.
- `an_unknown_model_id_is_an_error_not_a_silent_default` - `"medium.en"` (a real Whisper model this table deliberately does not carry) is rejected.

### Used by

- `src-tauri/src/asr/download.rs` and `src-tauri/src/asr/commands.rs` - `find`, `model_dir`, `url_for` and `is_installed`.
