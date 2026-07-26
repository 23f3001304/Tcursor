import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { startRecording, stopRecording, pauseRecording, resumeRecording, preprocessProject } from "../../lib/ipc";

export interface RecordingFlowDeps {
  micOn: boolean; micId: string | null; displayId: string | null;
  sysOn: boolean; gameMode: boolean; camOn: boolean;
  camStream: () => MediaStream | null;
  webcam: { start: (stream: MediaStream | null, folder: string) => void; stop: () => Promise<void> };
  onEdit?: (folder: string) => void;
  /** Written with the just-finished project's folder on Stop, so `Hud`'s (unrelated) export-error
   *  fallback listener can still reveal it - the ref is owned by the caller so both sides see it. */
  lastFolderRef: { current: string };
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

  // Registered once, well before any recording can finish, so there is no race between the
  // background preprocessing pass emitting its first event and the frontend subscribing to it
  // (the same reasoning `useEditorData`'s export-progress listener follows for exports).
  useEffect(() => {
    const unsubs: Promise<() => void>[] = [];
    unsubs.push(listen<number>("preprocess-progress", (e) => setSavePct(e.payload)));
    unsubs.push(listen("preprocess-done", () => preprocessDone.current?.()));
    unsubs.push(listen<string>("preprocess-error", () => preprocessDone.current?.()));
    return () => { unsubs.forEach((u) => u.then((f) => f())); };
  }, []);

  async function toggle() {
    if (saving) return;
    if (!recording) {
      setErr(null);
      let folder: string;
      try {
        folder = await startRecording(`rec-${Date.now()}`, micOn ? micId : null, displayId, sysOn, gameMode);
      } catch (e) {
        setErr(String(e));
        return;
      }
      if (camOn) webcam.start(camStream(), folder);
      setRecording(true);
    } else {
      // Immediate feedback: drop the recording UI the instant Stop is pressed, then finalize +
      // preprocess under a "Saving" state - see the module doc comment for why preprocessing is
      // awaited here (with progress) instead of racing the editor's mount on a detached thread.
      setRecording(false);
      setPaused(false);
      setSaving(true);
      setSavePct(0);
      try {
        // Screen finalize + the webcam's tail-chunk flush run in parallel - both quick now that the
        // webcam streamed to disk during recording (no big blob write left).
        const [res] = await Promise.all([stopRecording(), webcam.stop()]);
        lastFolderRef.current = res.folder;
        await new Promise<void>((resolve) => {
          preprocessDone.current = resolve;
          // If the invoke itself fails (vs. a `preprocess-error` event once the background pass is
          // actually running), resolve anyway rather than hanging "Saving..." forever - the editor's
          // own lazy `ensure_*` fallback still covers a project preprocessing never even started for.
          preprocessProject(res.folder).catch(() => resolve());
        });
        onEdit?.(res.folder); // open the editor onto the now-preprocessed project
      } catch (e) {
        setErr(String(e));
      } finally {
        setSaving(false);
        preprocessDone.current = null;
      }
    }
  }

  async function togglePause() {
    if (paused) { await resumeRecording(); setPaused(false); }
    else { await pauseRecording(); setPaused(true); }
  }

  return { recording, paused, saving, savePct, err, toggle, togglePause };
}
