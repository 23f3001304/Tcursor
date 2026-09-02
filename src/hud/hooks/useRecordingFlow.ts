import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { startRecording, stopRecording, pauseRecording, resumeRecording, preprocessProject } from "../../lib/ipc";
import { handOff } from "./handOff";

export interface RecordingFlowDeps {
  micOn: boolean; micId: string | null; displayId: string | null;
  sysOn: boolean; gameMode: boolean; camOn: boolean;
  camStream: () => MediaStream | null;
  webcam: { start: (stream: MediaStream | null, folder: string) => void; stop: () => Promise<void> };
  /** Promisified (fix round 1, item 2): MUST resolve only once the caller has fully taken over the
   *  window - see `./handOff.ts`'s doc comment for why. */
  onEdit?: (folder: string) => Promise<void>;
  /** Written with the just-finished project's folder on Stop, so `Hud`'s (unrelated) export-error
   *  fallback listener can still reveal it - the ref is owned by the caller so both sides see it. */
  lastFolderRef: { current: string };
}

/** One outcome of combining the screen-stop and webcam-stop `Promise.allSettled` results - the
 *  exact combinator semantics whose WRONG version (`Promise.all`) was bug H1. Screen failure is
 *  the only thing that fails the whole stop; a webcam-only failure is a soft warning carried as a
 *  field, never an extra code path, and never blocks the folder. Pure and exported so this
 *  branching is unit-tested directly (`useRecordingFlow.test.ts`) without mocking `stopRecording`/
 *  `webcam.stop` or rendering the hook. */
export type StopOutcome = { folder: string; camWarn?: string } | { err: string };

export function settleStop(
  screen: PromiseSettledResult<{ folder: string; frames: number }>,
  cam: PromiseSettledResult<void>,
): StopOutcome {
  if (screen.status === "rejected") return { err: `Recording failed: ${screen.reason}` };
  const camWarn = cam.status === "rejected"
    ? "Webcam track didn't finish — video saved without the camera overlay."
    : undefined;
  return { folder: screen.value.folder, camWarn };
}

/** Owns the record -> stop -> preprocess -> edit lifecycle, so `Hud` only wires UI to it. Stop
 *  drops the recording UI immediately (screen finalize + webcam flush run in parallel), then runs
 *  the FULL editor-preview preprocessing pass (`preprocess_project`: proxy/thumbs/waveforms/
 *  preview-audio/edit.json) and reports its progress as `savePct`, so the editor opens onto
 *  already-built artifacts instead of opening while that work is still racing along on a detached
 *  background thread (the old `thumbs::prewarm` behavior) - THAT race was why the preview could
 *  still take a while to load right after Stop. */
export function useRecordingFlow(deps: RecordingFlowDeps) {
  const { micOn, micId, displayId, sysOn, gameMode, camOn, camStream, webcam, onEdit, lastFolderRef } = deps;
  const [recording, setRecording] = useState(false);
  const [paused, setPaused] = useState(false);
  const [saving, setSaving] = useState(false); // finalizing + preprocessing, before the editor opens
  const [savePct, setSavePct] = useState(0);
  const [err, setErr] = useState<string | null>(null);
  // Set by toggle() right before kicking off preprocessing; called by the preprocess-done/-error
  // listener below to resolve the awaited promise. A ref (not state) so the listener - registered
  // once on mount, long before any recording ever finishes - always sees the current resolver.
  const preprocessDone = useRef<(() => void) | null>(null);
  // In-flight latch shared by `finish` AND `stopForClose`: only one of Stop, a backend-initiated
  // end, or a window-close can ever own the actual `stop_recording` call for a given take -
  // running the finalize twice would double-invoke it (the second returning "not recording").
  const finishing = useRef(false);
  // Re-entrancy guard for the OTHER slow half, `startRecording` (M2): it can take a second or two
  // (settings snapshot, spawning every tracker/thread, building the encoder), during which
  // `recording` is still false, so without this a fast double-click fires a second `start_recording`
  // that Rust rejects with "already recording" - painting a scary permanent banner over a take
  // that is actually running fine.
  const starting = useRef(false);
  // Re-pointed on every render so the mount-time listener below always calls the CURRENT finish
  // with the current `recording`/`saving` values, without re-subscribing.
  const endEarly = useRef<() => void>(() => {});

  // Registered once, well before any recording can finish, so there is no race between the
  // background preprocessing pass emitting its first event and the frontend subscribing to it
  // (the same reasoning `useEditorData`'s export-progress listener follows for exports).
  useEffect(() => {
    const unsubs: Promise<() => void>[] = [];
    unsubs.push(listen<number>("preprocess-progress", (e) => setSavePct(e.payload)));
    unsubs.push(listen("preprocess-done", () => preprocessDone.current?.()));
    unsubs.push(listen<string>("preprocess-error", () => preprocessDone.current?.()));
    // The take keeps running but something about it is degraded (an audio input that would not
    // open, so there is no narration). Surfacing it now is the whole point: the alternative is
    // the user finding out when the editor opens on a twenty-minute silent walkthrough.
    unsubs.push(listen<string>("record-warning", (e) => setErr(e.payload)));
    // The backend ended the take on its own - the recorded window closed, the display was
    // unplugged, or the recording's dimensions changed mid-take (window maximize/restore,
    // display resolution/rotation, dock/undock). The video already stopped there, so the only
    // wrong move is leaving the HUD counting: run the exact same finalize Stop runs, and say why.
    unsubs.push(listen<string>("record-ended-early", (e) => { setErr(e.payload); endEarly.current(); }));
    return () => { unsubs.forEach((u) => u.then((f) => f())); };
  }, []);

  /** Stops the Rust-side recording and flushes the webcam recorder IN PARALLEL (`allSettled`, not
   *  `all` - see `settleStop`), then applies the resulting `StopOutcome`: `err` fails the whole
   *  stop (nothing to hand off), `camWarn` alone still returns the folder but surfaces through the
   *  same `err`/banner slot as any other take-level message (fix round 1, item 2 - this used to
   *  be a bare `console.warn`, invisible in the running app). */
  async function stopCore(): Promise<{ folder: string } | null> {
    const [screen, cam] = await Promise.allSettled([stopRecording(), webcam.stop()]);
    const outcome = settleStop(screen, cam);
    if ("err" in outcome) { setErr(outcome.err); return null; }
    if (outcome.camWarn) setErr(outcome.camWarn);
    lastFolderRef.current = outcome.folder;
    return { folder: outcome.folder };
  }

  /** Stop -> finalize -> preprocess -> open the editor. See the module doc comment for why
   *  preprocessing is awaited here, and `./handOff.ts` for why `onEdit` is awaited too (fix
   *  round 1, item 2) instead of fired-and-forgotten. */
  async function finish() {
    if (finishing.current) return;
    finishing.current = true;
    // Immediate feedback: drop the recording UI the instant Stop is pressed, then finalize +
    // preprocess under a "Saving" state.
    setRecording(false);
    setPaused(false);
    setSaving(true);
    setSavePct(0);
    let resetSaving = true; // `finally` below resets `saving` unless `handOff` succeeds - see its doc
    try {
      const res = await stopCore();
      if (!res) return; // stopCore already surfaced the failure via `err`
      await new Promise<void>((resolve) => {
        preprocessDone.current = resolve;
        // If the invoke itself fails (vs. a `preprocess-error` event once the background pass is
        // actually running), resolve anyway rather than hanging "Saving..." forever - the editor's
        // own lazy `ensure_*` fallback still covers a project preprocessing never even started for.
        preprocessProject(res.folder).catch(() => resolve());
      });
      resetSaving = await handOff(onEdit, res.folder);
    } finally {
      if (resetSaving) setSaving(false);
      preprocessDone.current = null;
      finishing.current = false;
    }
  }

  /** Close-triggered variant of `finish` (R6 / task-6 (a)): stop + save only - no preprocessing,
   *  no editor hand-off, since the window is about to disappear. `Hud`'s Close button awaits this
   *  before actually closing the window, so a recording in progress is gracefully finalized
   *  (screen+mic+system-audio saved, webcam flushed) instead of the process just dying mid-take
   *  (the Critical finding this whole task starts from). Resolves immediately when there is
   *  nothing to do, and is a safe no-op if a stop is already in flight elsewhere (`finishing`) -
   *  the caller just proceeds to close once this resolves either way; Rust's own `CloseRequested`
   *  guard (`lib.rs`/`close_guard.rs`) is the backstop if THIS path never even runs (a hung
   *  renderer, or the OS close button bypassing the HUD's UI entirely). */
  async function stopForClose(): Promise<void> {
    if (!recording && !saving) return;
    if (finishing.current) return; // a Stop (or an earlier Close) already owns this take's stop
    finishing.current = true;
    setRecording(false);
    setPaused(false);
    setSaving(true);
    setSavePct(0);
    try {
      await stopCore();
    } finally {
      setSaving(false);
      finishing.current = false;
    }
  }

  endEarly.current = () => { if (recording && !saving) void finish(); };

  async function toggle() {
    if (saving) return;
    if (!recording) {
      if (starting.current) return; // re-entrancy guard (M2)
      starting.current = true;
      setErr(null);
      let folder: string | null = null;
      try {
        folder = await startRecording(`rec-${Date.now()}`, micOn ? micId : null, displayId, sysOn, gameMode);
        if (camOn) webcam.start(camStream(), folder);
        setRecording(true);
      } catch (e) {
        setErr(`Recording failed: ${e}`);
        if (folder) {
          // `startRecording` succeeded but something AFTER it threw (webcam.start - M1): Rust is
          // still actively recording with nothing on screen showing for it, and the only way back
          // in used to be Close, which destroyed the take (C1). Stop it instead.
          await stopRecording().catch(() => {});
        }
      } finally {
        starting.current = false;
      }
    } else {
      await finish();
    }
  }

  async function togglePause() {
    if (paused) { await resumeRecording(); setPaused(false); }
    else { await pauseRecording(); setPaused(true); }
  }

  return { recording, paused, saving, savePct, err, toggle, togglePause, stopForClose };
}
