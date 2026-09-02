import { useRef } from "react";
import { appendWebcam } from "../../lib/ipc";

/** Pure decision: given a `MediaRecorder`'s `state` (or `null` when there is no active recorder
 *  at all), should `stop()` attempt to call `mr.stop()`? `"inactive"` means the browser already
 *  auto-stopped the recorder (all tracks ended on their own) - calling `.stop()` again would throw
 *  `InvalidStateError` (task-6 (b) / finding H1), so that case (and no recorder) skips straight to
 *  flushing whatever `chain` already has queued. Exported/pure so this branch is unit-tested
 *  directly (`useWebcamRecorder.test.ts`) against every `RecordingState` without constructing
 *  a real `MediaRecorder`. */
export type WebcamStopAction = "skip" | "stop";
export function webcamStopAction(state: RecordingState | null): WebcamStopAction {
  return state === null || state === "inactive" ? "skip" : "stop";
}

/** Races `onDone` (the real `onstop` promise in `stop()` below) against a `timeoutMs` timer, so a
 *  `MediaRecorder` whose `onstop` never fires (browser quirk, or a stream torn down from elsewhere
 *  mid-call) can't hang the caller's whole Stop/Close flow forever (H1's secondary hazard). Split
 *  out from `stop()` purely for the timeout-path test seam - production always calls it with the
 *  real 4000ms budget. */
export function raceStopOrTimeout(onDone: Promise<void>, timeoutMs: number): Promise<"stopped" | "timeout"> {
  return Promise.race([
    onDone.then((): "stopped" => "stopped"),
    new Promise<"timeout">((res) => setTimeout(() => res("timeout"), timeoutMs)),
  ]);
}

// Streams the webcam to disk in 1s chunks DURING recording (MediaRecorder timeslices ->
// append_webcam) instead of holding the whole clip in memory and writing one O(length) blob at
// Stop. Appends are chained so chunks land in order; stop() awaits the tail so webcam.webm is
// complete before the editor opens.
export function useWebcamRecorder() {
  const rec = useRef<MediaRecorder | null>(null);
  const chain = useRef<Promise<void>>(Promise.resolve());
  const folder = useRef<string>("");

  function start(stream: MediaStream | null, dest: string) {
    if (!stream) return;
    folder.current = dest;
    chain.current = Promise.resolve();
    const mr = new MediaRecorder(stream, { mimeType: "video/webm" });
    mr.ondataavailable = (e) => {
      if (!e.data.size) return;
      const f = folder.current;
      chain.current = chain.current
        .then(() => e.data.arrayBuffer())
        .then((buf) => appendWebcam(f, new Uint8Array(buf)))
        .catch(() => {});
    };
    mr.start(1000); // 1s timeslice -> a chunk streamed to disk each second
    rec.current = mr;
  }

  /** Stops the webcam recorder and waits for its final chunk to flush - but a webcam failure can
   *  never take the whole Stop flow down with it (H1): if the tracks already ended on their own
   *  (unplug/driver reset/another app grabbing the device - see `useWebcamPreview`'s `ended`
   *  handling), the browser has already auto-stopped `mr` and a plain `mr.stop()` here would throw
   *  `InvalidStateError`, which used to propagate out of the caller's `Promise.all` and fail the
   *  ENTIRE stop (screen + mic + system audio all still finalized fine in Rust, but the editor
   *  never opened). Guarded below instead; the caller pairs this with `Promise.allSettled` so a
   *  rejection here is a soft failure, not a hard one. */
  async function stop(): Promise<void> {
    const mr = rec.current;
    // Cleared up front, before any call that can throw, so a failed/auto-stopped recorder can
    // never linger and wedge the NEXT start/stop cycle with the same corpse.
    rec.current = null;
    if (webcamStopAction(mr?.state ?? null) === "skip") { await chain.current; return; }
    const mrLive = mr as MediaRecorder; // "skip" above is the only `null` case
    const stopped = new Promise<void>((res) => { mrLive.onstop = () => res(); });
    try {
      mrLive.stop(); // fires a final ondataavailable (queues its append), then onstop
    } catch {
      // Raced: tracks ended between the state check above and this call. `onstop` will never
      // fire for a call that threw - whatever chunk the browser's own auto-stop already queued
      // is still in `chain`, so just flush that and return.
      await chain.current;
      return;
    }
    await raceStopOrTimeout(stopped, 4000);
    await chain.current; // all chunks flushed to disk
  }

  return { start, stop };
}
