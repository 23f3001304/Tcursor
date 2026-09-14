# src-tauri/src/session/record/switch_mic.rs

Mid-take microphone switching (2026-09-14; design: `docs/superpowers/specs/2026-09-14-mid-take-source-switching-design.md`, plan: `docs/superpowers/plans/2026-09-14-mid-take-source-switching.md` Task M).

A `cpal::Stream` is `!Send`, lives on the thread that built it, and writes one WAV for its whole life, so there is no such thing as pointing the running capture at another device. A switch is therefore a thread swap: the current mic thread is told to stop, joined (which finalizes its WAV), and a new one is spawned onto a NEW file. Each new file is a `segments::Segment` in the take's `SegmentLog`, and `export::preview::segments_audio::merge_mic_segments` folds every one of them back into the single `mic.wav` during `preprocess::essential`, before the editor opens. Nothing downstream of preprocess ever learns a switch happened.

## switch_mic

```rust
#[tauri::command]
pub fn switch_mic(
    device_id: Option<String>,
    recorder: tauri::State<'_, Recorder>,
    app: tauri::AppHandle,
) -> Result<(), String>
```

Switch the running take's microphone. The HUD's Sources sheet calls this (via `ipc.ts`'s `switchMic`) whenever the user picks another input mid-take.

### Inputs

- `device_id: Option<String>` - the cpal input device name to record from here on, in the same spelling `commands::list_audio_inputs` hands the HUD and `start_recording` takes. `None` means **mic off from here on**: no thread is spawned, no file is written, but the boundary is still logged (see Implementation step 5).
- `recorder: State<Recorder>` - the managed recording state. The whole switch runs under its lock.
- `app: AppHandle` - used only to rebuild the two sinks the mic thread reports through: `emit::emitter(&app, "record-warning")` and `emit::level_emitter(&app, "mic")`.

### Returns

`Ok(())` once the new thread (or the deliberate absence of one) owns the take's `mic_stop`. `Err("not recording")` when no take is running - the same wording `pause_recording`, `resume_recording` and `stop_recording` use for that state, so the HUD has one message for it.

### Implementation

1. Lock `Recorder::inner` and take `&mut Running`, or return `Err("not recording")`. *Why the whole switch is under one lock:* a second `switch_mic`, or a Stop, must never observe the take between "the old thread is gone" and "the new one owns `mic_stop`". That window is exactly where a take could end up with two live mic streams writing at once, or with none and no segment recorded.
2. `mic_stop.store(true)` and join `mic_thread`. *Why `mic_stop` and not the take's `stop`:* `stop` ends the system-audio thread and the video pipeline too; `Running.mic_stop` (added by Task 0) exists so only the mic's own thread hears this. *Why join rather than detach:* `CpalMicHandle::stop` is what makes hound rewrite the RIFF/data sizes, so only a joined thread has left a complete, probeable WAV for the merge to read a spec and a duration from.
3. Stamp `segments::recording_ms(clock, paused_totals)` **after** the join, so the recorded instant is where the new segment really begins (the join costs up to `LEVEL_POLL_MS`) rather than where the request arrived.
4. Name the file with `segments::next_name("mic", "wav", log.mic.len())` - the log holds only the EXTRA segments, so the first switch opens `mic_2.wav` - and spawn `recorder_threads::spawn_mic_thread` with a **fresh** `stop` flag (stored back into `Running.mic_stop`) and a **fresh** `started` counter. *Why not `Running.mic_start`:* that atomic is `sync.json`'s `mic_ms`, which places the FIRST segment and which the merge measures every later segment's delay from; overwriting it would move the whole track. A later segment's own first-sample stamp has no reader, so it is dropped.
5. Push `Segment { path, start_ms }` into `Running.segments.mic` - **including** when `device_id` was `None` and no thread was spawned. *Why log a boundary with no file:* it keeps the numbering of the next switch honest (`mic_3.wav` after a muted `mic_2.wav`), and `segments_audio::parts_of` simply finds nothing at that path and skips it, which leaves the silence the mute was asking for.

### Why the level meter keeps working

The new thread is given `level_emitter(&app, "mic")`, byte for byte the sink `start_recording` builds. The HUD's `useAudioLevels` treats a source with no report for 400ms as no longer capturing, so a switch shows as a brief dip and then the new device's own levels - and a switch to `None`, or to a device that will not open, correctly shows as no meter at all, because `spawn_mic_thread` drops the sink when the device does not open.

### Behaviors

- `each_switch_opens_the_next_numbered_wav`: driving `next_name` off the log the command pushes into yields `mic_2.wav`, `mic_3.wav`, `mic_4.wav`; a muted switch still occupies its number. (The command itself needs a live Tauri app and real capture devices to run, so this pins the naming rule it applies rather than the command call.)
