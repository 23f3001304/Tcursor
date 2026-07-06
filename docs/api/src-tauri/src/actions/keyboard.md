# src-tauri/src/actions/keyboard.rs

Tracks hotkey chords and typing activity by polling the Windows async key state at ~60 Hz, instead of installing a `WH_KEYBOARD_LL` hook that security software silently swallows on some machines. Runs on a dedicated background thread owned by `KeyboardTracker`; all cross-thread state is `Arc`-wrapped. Non-Windows builds compile but the poll loop never fires, so the tracker is inert on those platforms.

## TYPING_VKS

```rust
const TYPING_VKS: &[u32]
```

The set of Win32 virtual-key codes that count as "typing": digits `0x30-0x39`, letters `0x41-0x5A`, space `0x20`, Enter `0x0D`, Backspace `0x08`, Tab `0x09`. *Why:* only fresh down-transitions on these keys are timestamped - which key is never stored, preserving user privacy while giving the AI director a typing cadence to extend zoom holds with.

## KeyboardTracker

```rust
pub struct KeyboardTracker {
    stop: Arc<AtomicBool>,
    events: Arc<Mutex<Vec<ActionEvent>>>,
    typing: Arc<Mutex<Vec<u32>>>,
    thread: Option<JoinHandle<()>>,
}
```

Owns a background `"keyboard-poll"` thread that reads physical key state directly. Each field is `Arc`-wrapped so the thread closure can share state with the struct without lifetime coupling.

- `stop` - atomic flag the poll loop checks each iteration; `store(true)` is the shutdown signal. *Why:* an `AtomicBool` with `SeqCst` ordering is safe to write from outside the thread without a mutex.
- `events` - accumulates `ActionEvent` values (hotkey presses/releases) in arrival order. *Why:* separating hotkey events from typing timestamps lets callers pass them to different consumers (`ai::timeline`, `export::autozoom`) without re-partitioning.
- `typing` - accumulates millisecond timestamps of fresh key-down events on `TYPING_VKS`. *Why:* only the timestamp is needed downstream - the key identity is intentionally discarded.
- `thread` - `Option<JoinHandle<()>>` so `Drop` can `take()` the handle and join without double-join risk.

### Used by

- `src-tauri/src/session/record/recorder.rs` - stores a `KeyboardTracker` in the `Recorder` struct and creates it via `KeyboardTracker::start(arming_from_settings(&snap.hotkeys))`.
- `src-tauri/src/session/record/recorder_threads.rs` - accepts `Option<KeyboardTracker>`, calls `stop()` when the session ends, and writes the returned events to `actions.json`.

## KeyboardTracker::start

```rust
pub fn start(arms: Vec<Arm>) -> Self
```

Spawns the poll thread and starts accumulating data immediately.

### Inputs

- `arms: Vec<Arm>` - the armed hotkey table from `arming_from_settings`. *Why:* the thread only polls the VK codes actually in this table plus `TYPING_VKS`, so it is not a general keylogger - only the user-configured chords are inspected.

### Implementation

1. Allocate shared state: `stop` (`AtomicBool::new(false)`), `events` and `typing` as `Arc<Mutex<Vec<_>>>`.
2. Clone all three `Arc`s for the thread closure - the originals stay in `Self` for `stop`/`drop`.
3. Spawn `"keyboard-poll"` thread. *Why name it:* named threads appear in debuggers and panic messages, aiding diagnosis.
4. Inside the thread: record `Instant::now()` as the epoch so all timestamps are session-relative. Initialize `active` (per-arm held-state) and `typ_prev` (per-typing-key prior state), both as `vec![false; N]`.
5. Loop on `!s.load(SeqCst)`:
   - Read `cur_mods()` (VK_CONTROL/MENU/SHIFT via `GetAsyncKeyState`).
   - For each arm: if `key_down(arm.chord.vk) && mods == arm.chord.mods` changed since the last poll, record an `ActionEvent`. *Why:* level-triggered polling combined with an `active` array gives edge detection (transition, not continuous state) without a kernel hook.
   - For each TYPING_VK: if the key just went down (was up last poll), push the timestamp to `typing`. *Why fresh-down only:* suppresses auto-repeat so one held key does not flood the typing log.
   - Sleep 15 ms (~66 Hz). *Why:* fast enough to catch short key presses but slow enough not to waste CPU on a recording host.
6. If `spawn` fails, `thread` is `None`; the tracker exists but records nothing. *Why tolerate this:* a recording should not abort just because the keyboard thread failed to start.

### Returns

`Self` with the background thread running. All timestamps in the returned vectors will be relative to the moment `start` was called.

### Behaviors

- `stop_returns_tuple_with_empty_vecs_when_no_input` - starting with an empty arms list and immediately stopping yields two empty vecs; validates the round-trip without hardware input.

## KeyboardTracker::stop

```rust
pub fn stop(mut self) -> (Vec<ActionEvent>, Vec<u32>)
```

Shuts down the poll thread and harvests accumulated data.

### Implementation

1. `stop.store(true, SeqCst)` - signals the loop to exit on the next iteration (at most 15 ms later).
2. `thread.take().join()` - waits for the thread to fully exit before touching the mutexes. *Why join before drain:* ensures no concurrent push can race the `mem::take` below.
3. `mem::take` on both locked vecs returns the data and leaves empty vecs behind, so `Self` holds nothing after the call. *Why `mem::take` rather than `clone`:* avoids a redundant heap allocation.
4. `unwrap_or_else(|e| e.into_inner())` on poisoned mutexes - recovers the data even if the poll thread panicked mid-push. *Why:* a panicked thread should not silently discard all recorded events.

### Returns

`(Vec<ActionEvent>, Vec<u32>)` - hotkey events and typing timestamps, both in ascending time order (insertion order equals chronological order because the poll loop is single-threaded).

## KeyboardTracker::drop

```rust
fn drop(&mut self)
```

Safety-net `Drop` impl: if the tracker is dropped without calling `stop` (e.g. an error path in `start_recording` that returns early), sets the stop flag and joins the thread to prevent a dangling background thread that holds `Arc` references to the recording's data buffers.
