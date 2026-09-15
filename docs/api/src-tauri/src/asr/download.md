# src-tauri/src/asr/download.rs

Fetching a Whisper model file, resumably and honestly.

Two rules shape the whole file. First, a half-finished download is kept as `<file>.part` and picked up where it stopped - `small.en` is 465 MiB, and a user on a hotel connection should not start over because the app was closed. Second, nothing is ever moved onto the final path until its sha256 matches the table, so a truncated or tampered file can never become the model the transcriber loads.

The decision of what to do with an existing `.part` is a pure function (`resume_from`), which is how the three interesting cases get tested without touching the network. No test in this file makes an HTTP request.

HTTP goes through `ureq` 2, which the crate already depends on for the Ollama client - no new dependency, and the same blocking, no-async-runtime shape the rest of this codebase uses for outbound calls.

## Resume

```rust
#[derive(PartialEq, Eq, Debug)] pub enum Resume { Complete, Range(u64), Restart }
```

What an existing `.part` is worth. `Complete` means it is already the whole file and only needs verifying and renaming; `Range(n)` means ask the server for byte `n` onward; `Restart` means truncate and start over.

## resume_from

```rust
pub fn resume_from(part_len: u64, total: u64) -> Resume
```

The decision, from the two lengths alone. Zero bytes restarts (there is nothing to resume). A part LONGER than the file restarts too: that can only be junk from an interrupted write or a file that changed upstream, and appending to it would produce a file that fails its digest after another 465 MiB of traffic.

### Behaviors

- `resume_decides_from_the_part_length_alone` - pins all four cases: 0, a genuine partial, an exact match, and an over-long part.

## verify_or_remove

```rust
pub fn verify_or_remove(path: &Path, want_sha256: &str) -> Result<(), String>
```

Hash `path` and keep it only if the digest matches. A mismatch DELETES the file and returns a message naming the file and BOTH digests.

*Why delete:* a bad file left on disk is worse than no file. `is_installed` would report it as present, the panel would stop offering the download, and the next transcription would fail inside whisper.cpp. Removing it puts the user back in a state the Download button can fix.

*Why say both digests:* "the file is corrupt" is unactionable. The expected-and-got pair is the one piece of evidence that distinguishes a truncated transfer from a model table that has drifted from what Hugging Face now serves.

A file that cannot be read at all is an error that names it, not a panic - the downloader calls this on a `.part` it believes it just wrote, and a disk that lost it must reach the panel as a message.

### Behaviors

- `a_file_whose_digest_is_wrong_is_deleted_and_named_in_the_error` - the error carries the actual digest and the file is gone afterwards.
- `a_file_whose_digest_matches_is_kept` - the happy path does not delete.
- `a_missing_file_is_an_error_that_names_it_rather_than_a_panic` - the read failure is a `Result`, and the message contains the filename.

## finish

```rust
fn finish(part: &Path, target: &Path, spec: &ModelSpec) -> Result<(), String>
```

Verify the `.part` and rename it onto the model path. Private, and the ONLY route from `.part` to a real model file - which is what makes "nothing unverified is ever installed" a property of the file rather than a habit of its callers.

## download_model

```rust
pub fn download_model(
    spec: &ModelSpec,
    on_progress: &dyn Fn(u64, u64),
    cancel: &dyn Fn() -> bool,
) -> Result<PathBuf, String>
```

Download `spec` into `model_dir()`, resuming an interrupted attempt when one is on disk. Blocking; the caller owns the thread. Returns the final path.

The order of operations:

1. Create the model directory.
2. If the final file is already there AND verifies, report 100% and return it. A final file that does NOT verify is deleted by `verify_or_remove` and the download proceeds, which is how a model corrupted after installation repairs itself.
3. Ask `resume_from` about any `.part`. `Complete` short-circuits to verify-and-rename with no network at all.
4. GET the URL, with `Range: bytes=<n>-` when resuming. A refused range (a 416 once the file upstream changed, say) is not fatal: it starts over from zero rather than failing.
5. Only a **206** actually honoured the range. Any other success status is the whole file again, so `done` resets to zero and the `.part` is truncated rather than appended to - the bug this guards against silently produces a file of the right length made of the wrong bytes, which then fails its digest with no explanation.
6. Copy in 256 KiB reads, reporting progress at most every 64 KiB of new data and polling `cancel()` on every read.
7. Flush, close, verify, rename, report 100%.

*Why a cancel LEAVES the `.part` behind:* that is the entire point of resuming. A user who cancels a 465 MiB download at 80% and starts it again tomorrow should pay for 20%, not 100%.

*Why progress is throttled:* every call crosses a Tauri event boundary into the webview. At 256 KiB a read an unthrottled `small.en` would emit ~1900 events; the 64 KiB floor keeps the bar smooth without flooding the frontend.

`bytes` from the table, not the response's `Content-Length`, is the progress denominator: on a resumed download the header describes only the remaining range, and a bar that restarts at zero for the last 20% is a lie.

### Used by

- `src-tauri/src/asr/commands.rs` (`download_whisper_model`) - the only caller; it supplies an `on_progress` that emits `asr-download-progress` and, for now, a `cancel` that is always `false`.
