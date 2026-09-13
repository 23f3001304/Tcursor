# setup/src-tauri/src/log.rs

The text behind the error screen's "Copy log", and the file behind the path it names.

An install that fails is the one moment where a user has to tell somebody else what happened, so
the log exists to be pasted into a message: payload path and size, destination and expected bytes,
exit code, stderr tail, how many bytes actually landed. It is a `Vec<String>` in memory with a
mirror at `%TEMP%\TCursorSetup.log`, written at every end state so it survives the window closing.

Stamps are seconds since the window opened rather than wall-clock time, because formatting a local
date without a calendar crate is a dozen lines of civil-time arithmetic for something a support
reader does not need. The first line carries the Unix timestamp, which anchors the relative ones.

## buffer

```rust
fn buffer() -> &'static Mutex<Vec<String>>
```

The lines, seeded on first touch with a header naming the Setup version and the Unix time it
started. A poisoned lock is treated as "no log" rather than a panic, in both `line` and `text`: the
log is diagnostic, and losing it must not take the installer with it.

## started

```rust
fn started() -> Instant
```

The zero of the timestamps. `main` calls `line` before building the Tauri app so this is set at
process start, not at the first interesting event.

## path

```rust
pub fn path() -> PathBuf
```

`%TEMP%\TCursorSetup.log`. Fixed, not per-pid: the error screen prints this path, and a user going
to look for it should find one file, not a pile of them. A second run overwrites it.

## line

```rust
pub fn line(message: impl AsRef<str>)
```

Appends one stamped line.

## text

```rust
pub fn text() -> String
```

The whole log, CRLF-joined so it opens correctly in Notepad and pastes cleanly on Windows.

## flush

```rust
pub fn flush()
```

Writes `text()` to `path()`, ignoring failures. Called at every end state (success, failure,
abandonment, a launch that would not start) rather than on every line, so the common case is one
write instead of one per poll.
