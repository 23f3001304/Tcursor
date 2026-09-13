# setup/src-tauri/src/install.rs

The Setup window's whole back end: the embedded NSIS payload, the commands `app.js` calls, and the
worker thread that runs the installer and reports on it.

The division of labour is deliberate. Every decision - the percent, the wording, the destination,
the version, whether a close is allowed, what goes in the log - is made in Rust and handed to the
UI already made. `setup/ui/app.js` has no policy in it, because there is no test runner in front of
it.

Two consts are compiled in from `build.rs`: `NSIS_PAYLOAD` (`$OUT_DIR/nsis-setup.exe`, the real
installer or an empty placeholder in a dev build) and `LICENSE` (`$OUT_DIR/eula.txt`, the project's
`eula.rtf` flattened to text). Two statics carry the only mutable state: `RUNNING`, which gates
double-clicks and tells `main.rs` whether a close needs a confirmation, and `CLOSE_ALLOWED`, which
the confirmation sets.

## Done

```rust
struct Done { ok: bool, message: String }
```

Payload of the `install://done` event. `message` is empty on success.

## Info

```rust
pub struct Info { version: String, destination: String, log_path: String, expected_mb: u64 }
```

The fine print, fetched once at startup. `expected_mb` is 0 when this build could not measure the
payload, and the welcome line about the download size is then simply not shown - better than
printing "About 0 MB".

## setup_info

```rust
#[tauri::command]
pub fn setup_info() -> Info
```

Everything the rail and the welcome screen need. Called once; the values cannot change while the
window is open.

## license_text

```rust
#[tauri::command]
pub fn license_text() -> &'static str
```

The license as plain text, for the in-window sheet behind "By installing you agree to the license".

*Why an in-window sheet and not the OS handler:* `eula.rtf` is not on the user's disk before the
install (it is a page in the NSIS wizard, which `/S` skips, and it is not copied into `$INSTDIR`).
Showing it externally would mean writing a temp file and handing it to whatever owns `.rtf`, which
is a context switch and possibly an "how do you want to open this" dialog on a machine with no RTF
handler. The sheet is one scroll region in a window the user already has.

*Why the real file and not a hand-written summary:* a summary in the Setup UI is a second license
text that can drift from the one the wizard shows. This is the same bytes, flattened at build time.

## copy_log

```rust
#[tauri::command]
pub fn copy_log() -> String
```

The accumulated log, for the error screen's "Copy log". Flushes to `%TEMP%\TCursorSetup.log` on the
way out, so the path named in the window is current at the moment the user is told about it.

The clipboard write itself is done in the webview (`app.js`), because this crate has no clipboard
dependency and the brief forbids adding one.

## allow_close

```rust
#[tauri::command]
pub fn allow_close(window: tauri::Window)
```

The "Yes" of the mid-install confirmation. Sets `CLOSE_ALLOWED`, records the abandonment in the
log, and closes.

Note what it does **not** do: kill the installer. The NSIS child is detached and silent, and
killing it partway through would leave a half-written install with no uninstaller entry. Closing
the window abandons the progress display, not the install, and the log says so.

## busy

```rust
pub fn busy() -> bool
```

Is an install in flight? Read by `main.rs`'s close guard.

## close_allowed

```rust
pub fn close_allowed() -> bool
```

Has the user already answered the confirmation? Read by the same guard, so the second close request
is not intercepted again.

## start_install

```rust
#[tauri::command]
pub fn start_install(app: AppHandle) -> Result<(), String>
```

Stages the payload and spawns the worker. Returns as soon as the install is under way; the UI then
follows `install://tick` and `install://done`.

Re-entry is a no-op rather than an error: `RUNNING.swap(true)` means a second click on Install (or
on Retry before the first attempt has finished) is swallowed instead of starting a second
installer. The only error path is staging, which is reported synchronously so the button press
itself can fail.

## stage

```rust
fn stage() -> Result<PathBuf, String>
```

Writes `NSIS_PAYLOAD` to `%TEMP%\TCursor_setup_<pid>.exe`, and does nothing if that file is already
there.

*Why keyed on the pid and reused:* Retry must re-run the install "from the staged payload". Writing
58MB again on each attempt is the obvious way to get that wrong, so `finish` deletes the file only
on success, and a retry within the same process finds it and runs it straight away.

The file handle is dropped before the function returns; executing a file we still hold open fails
on Windows.

A payload under 1KB is the dev-build placeholder, and is refused here with a message naming
`setup/build.ps1` rather than being launched.

## run

```rust
fn run(app: AppHandle, payload: PathBuf)
```

The worker thread. Spawns the payload with `/S`, then polls every 150ms: measure the destination,
map the measurement through `progress::step`, emit a tick. On exit it records the code, the final
byte count and any stderr, and hands the outcome to `finish`.

`peak` holds the running maximum percent. An upgrade uninstalls the previous version first, so the
measured folder size drops partway through, and without this the bar would visibly reverse.

stderr is piped so the log can carry a tail of it; stdout is discarded. NSIS `/S` is silent on
both, so this is for the failure case, and the pipe is only read after the child has exited (a
4KB-plus pipe buffer against an installer that writes nothing is not going to deadlock).

## tail

```rust
fn tail(stderr: Option<std::process::ChildStderr>) -> String
```

Last 400 characters of the child's stderr, trimmed, or empty. Sliced on a character boundary so a
truncated UTF-8 sequence cannot panic the worker.

## finish

```rust
fn finish(app: &AppHandle, payload: &PathBuf, outcome: Result<(), String>)
```

The single exit point: clears `RUNNING`, writes the log to disk, and emits `install://done`.

On success it deletes the staged payload and emits one last tick at 100. On failure it keeps the
payload, because Retry is about to want it.

## emit

```rust
fn emit(app: &AppHandle, step: Step)
```

Sends a `Step` as `install://tick`. Emit failures are ignored: the window closing mid-install is a
normal way for this to fail, and there is nobody left to tell.

## launch_app

```rust
#[tauri::command]
pub fn launch_app() -> Result<(), String>
```

Runs `<destination>\tcursor-scaffold.exe`. Called twice over: by the "Open TCursor" button, and by
the ~1s timer the Done screen starts. `app.js` closes the window on success and falls into the
error state on failure, so a machine where the launch is blocked keeps a window with a message and
a log rather than vanishing.
