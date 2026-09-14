import { useState } from "react";
import { markWebcamSegment, switchDisplay as switchDisplayCmd, switchMic as switchMicCmd } from "../../lib/ipc";

/** Polls `get` every `stepMs` until `ok` accepts what it returns, giving up after `timeoutMs`
 *  (resolving `null`). The camera switch needs it because `useWebcamPreview` re-acquires the
 *  device through an effect, not a promise the caller can await: the only signal that the new
 *  device is live is its stream appearing in that hook's ref. Pure and exported so the give-up
 *  path is unit-tested with fake timers instead of an unplugged webcam. */
export async function pollFor<T>(get: () => T, ok: (v: T) => boolean, timeoutMs: number, stepMs = 100): Promise<T | null> {
  for (let waited = 0; waited <= timeoutMs; waited += stepMs) {
    const v = get();
    if (ok(v)) return v;
    await new Promise((r) => setTimeout(r, stepMs));
  }
  return null;
}

/** How long a newly picked camera has to produce a stream before the switch gives up and says so.
 *  Generous: a cold USB camera can take a second or two to hand over its first frame. */
export const CAMERA_WAIT_MS = 5000;

export interface SourceSwitchDeps {
  recording: boolean;
  /** The take's own toggles: a source that is OFF for this take is only pre-selected for the next
   *  one, never switched live - there is no first segment on disk for a merge to extend. */
  camOn: boolean; micOn: boolean;
  camId: string | null; camStream: () => MediaStream | null;
  webcam: {
    start: (stream: MediaStream | null, folder: string, segment?: number) => void;
    stop: () => Promise<void>; folder: () => string; segment: () => number;
  };
  setCamId: (id: string | null) => void;
  setMicId: (id: string) => void;
  setDisplayId: (id: string) => void;
}

/** The three mid-take source switches behind the take pill's Sources sheet (2026-09-14 design:
 *  `docs/superpowers/specs/2026-09-14-mid-take-source-switching-design.md`). Each one applies to a
 *  RUNNING take and to the HUD's own selection; while idle they are plain selection changes, so
 *  the sheet's pickers behave the same before and during a take.
 *
 *  Failures never end the take: they land in `err`, which `Hud` shows through the take pill's
 *  existing warning slot, and the recording carries on with whatever it already had. */
export function useSourceSwitch(d: SourceSwitchDeps) {
  const [err, setErr] = useState<string | null>(null);

  /** The camera is the only switch the webview owns: a `MediaRecorder` cannot change its stream,
   *  so the running recorder is flushed, the preview re-acquires the new device, and a SECOND
   *  recorder opens `webcam_<n>.webm` after `mark_webcam_segment` stamps where it starts.
   *  `null` is the camera going off mid-take: flush and stop, and mark nothing (there is no
   *  segment to place). */
  async function switchCamera(id: string | null) {
    if (id !== null && id === d.camId) return;
    if (!d.recording || !d.camOn) { d.setCamId(id); return; }
    const prev = d.camStream()?.id ?? null;
    const dest = d.webcam.folder();
    const next = d.webcam.segment() + 1;
    await d.webcam.stop(); // the current segment's tail reaches disk before the new one opens
    d.setCamId(id);
    if (id === null || !dest) return;
    const stream = await pollFor(d.camStream, (s) => s != null && s.id !== prev, CAMERA_WAIT_MS);
    if (!stream) { setErr("The new camera did not start. The take is continuing without it."); return; }
    try {
      await markWebcamSegment(next);
    } catch (e) {
      setErr(`Camera switch failed: ${e}. The take is continuing without it.`);
      return;
    }
    d.webcam.start(stream, dest, next);
  }

  /** Rust finalizes the running WAV and starts `mic_<n>.wav` on the new device; the HUD's level
   *  meter follows the new capture thread on the same `audio-level` event. */
  async function switchMic(id: string) {
    if (!d.recording || !d.micOn) { d.setMicId(id); return; }
    try { await switchMicCmd(id || null); d.setMicId(id); }
    catch (e) { setErr(`Microphone switch failed: ${e}`); }
  }

  /** Rust restarts the capture on the new target into the SAME encoder canvas (and remaps mouse
   *  coordinates), so the take never changes size and the editor never learns a second screen. */
  async function switchDisplay(id: string) {
    if (!d.recording) { d.setDisplayId(id); return; }
    try { await switchDisplayCmd(id); d.setDisplayId(id); }
    catch (e) { setErr(`Display switch failed: ${e}`); }
  }

  return { err, clearErr: () => setErr(null), switchCamera, switchMic, switchDisplay };
}
