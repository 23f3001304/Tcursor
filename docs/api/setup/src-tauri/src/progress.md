# setup/src-tauri/src/progress.rs

Where the Setup window gets its numbers. NSIS run with `/S` prints nothing and exposes no progress
channel, so instead of believing the installer we watch its effect: how many bytes have appeared in
the install folder, against how many the bundler said it would write.

Three things live here: the destination (derived, not guessed), a folder-size probe, and the pure
mapping `step` that turns an observation into the percent, phase and status line the window shows.
Everything is deterministic and side-effect free apart from `dir_bytes`, so the behaviour is pinned
by unit tests rather than by watching a real install.

`EXPECTED_BYTES` is not declared in this file. `build.rs` writes
`$OUT_DIR/expected_bytes.rs` (`pub const EXPECTED_BYTES: u64 = ...`) and this module `include!`s it,
so the constant is baked per build. `0` means this build could not measure the payload; see `step`
for what happens then, and `setup/src-tauri/build.rs` for where the number comes from.

## MAIN_BINARY

```rust
pub const MAIN_BINARY: &str = "tcursor-scaffold.exe";
```

The file NSIS drops in `$INSTDIR`, and therefore what "Open TCursor" runs. It is `MAINBINARYNAME`
in the generated `installer.nsi`, which Tauri takes from the main crate's Cargo package name
(`tcursor-scaffold`), **not** from `productName`. If the main crate is ever renamed, or
`mainBinaryName` is set in `src-tauri/tauri.conf.json`, this constant moves with it.

## destination

```rust
pub fn destination() -> PathBuf
```

`%LOCALAPPDATA%\TCursor`.

*Why that path and not `%LOCALAPPDATA%\Programs\...`:* `src-tauri/tauri.conf.json` sets
`bundle.windows.nsis.installMode` to `currentUser`. The generated script then defines
`MULTIUSER_INSTALLMODE_INSTDIR` as `${PRODUCTNAME}` and includes `MultiUser.nsh`, which resolves
`$INSTDIR` to `$LOCALAPPDATA\<productName>` for a current-user install. `Programs\` is the
Squirrel/Electron convention, not Tauri's.

This is both the folder we poll and the folder we launch from, so a wrong answer here would show as
a bar stuck at "Preparing" rather than as a crash - which is exactly why the fallback in `step`
exists.

## dir_bytes

```rust
pub fn dir_bytes(root: &Path) -> u64
```

Sum of every file under `root`, and `0` if `root` does not exist yet (which is the normal state
before the first install). Iterative with an explicit stack, so a deep tree cannot blow the
worker thread's stack; errors on individual entries are skipped rather than propagated, because a
file NSIS is mid-write is a transient, not a failure.

Called once per 150ms poll against a folder holding a few dozen files.

## Phase

```rust
pub enum Phase { Preparing, Installing, Registering, Installed }
```

Serialized lowercase to the UI, which uses it for the progress bar's `data-phase` (only
`preparing` gets the indeterminate shimmer, because that is the one state where the number is
genuinely unknown). The phases are observations, not a script: `Registering` means the file bytes
are all present and the child process is still alive, which in an NSIS run is the registry keys,
the shortcuts, the file association and the uninstaller.

## Step

```rust
pub struct Step { pub percent: u32, pub phase: Phase, pub status: String }
```

One tick of `install://tick`. `status` is the exact sentence the window prints; the wording lives
here rather than in `app.js` so it can be tested and so the UI stays a painter.

## step

```rust
pub fn step(bytes: u64, expected: u64, child_running: bool, elapsed_ms: u64) -> Step
```

The whole progress policy, as one pure function:

| Condition | percent | phase | status |
|---|---|---|---|
| `!child_running` | 100 | Installed | `Installed` |
| `expected == 0` | `min(92, 3 + 2 * elapsed_ms / 220)` | Installing | `Installing files` |
| `bytes == 0` | 0 | Preparing | `Preparing` |
| `bytes >= expected` | 97 | Registering | `Registering shortcuts` |
| otherwise | `min(97, bytes * 100 / expected)` | Installing | `Installing files, 12 of 38 MB` |

*Why 97 and not 99:* the cap is there so the bar cannot claim completion the installer has not
reported. Copying files is most of the wall time but not all of it, and the remaining registry and
shortcut work has no measurable footprint, so the bar parks at a number that visibly is not done
and the exit code is what moves it to 100.

*Why `!child_running` means success:* a non-zero exit or a spawn failure never reaches this
function. `install.rs` sends those straight to the error state, so the only way to observe a
finished child here is a clean exit.

*The `expected == 0` row is the old behaviour, kept deliberately.* Before this milestone the bar
was paced on a timer: start at 3, add 2 every 220ms, stop at 92. That is decorative progress, and
it is now reachable only when there is nothing to measure - a dev build with no NSIS payload
staged, or a tree where the bundler's staging folder was pruned. Two tests pin it so it cannot be
deleted by accident and cannot creep back into the measured path.

*Why `elapsed_ms` is a parameter at all* when four of the five rows ignore it: passing the clock in
keeps the function pure, so the fallback's pacing is testable at chosen instants instead of by
sleeping. `elapsed_time_is_ignored_once_the_size_is_known` pins that it changes nothing when
`expected > 0`.

Not handled here, on purpose: the bar never walking backwards. An upgrade deletes the old files
before writing the new ones, so `bytes` can dip. `install.rs` keeps the running maximum, because
"what did we last show" is state and this function has none.

## mb

```rust
fn mb(bytes: u64) -> u64
```

Mebibytes rounded to nearest, which is what Windows labels "MB". `step` floors the current count at
1 once any byte has landed, so the first poll reads `1 of 219 MB` rather than `0 of 219 MB`.
