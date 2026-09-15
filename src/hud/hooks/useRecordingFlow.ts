import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  startRecording,
  stopRecording,
  pauseRecording,
  resumeRecording,
  preprocessProject,
} from "../../shared/ipc";
import { handOff } from "./handOff";

export interface RecordingFlowDeps {
  micOn: boolean;
  micId: string | null;
  displayId: string | null;
  sysOn: boolean;
  gameMode: boolean;
  camOn: boolean;
  camStream: () => MediaStream | null;
  webcam: { start: (stream: MediaStream | null, folder: string) => void; stop: () => Promise<void> };
  onEdit?: (folder: string) => Promise<void>;
  lastFolderRef: { current: string };
}

export type StopOutcome = { folder: string; camWarn?: string } | { err: string };

export function settleStop(
  screen: PromiseSettledResult<{ folder: string; frames: number }>,
  cam: PromiseSettledResult<void>,
): StopOutcome {
  if (screen.status === "rejected") return { err: `Recording failed: ${screen.reason}` };
  if (screen.value.frames === 0)
    return { err: "No video was captured. Nothing reached the screen encoder; try the take again." };
  const camWarn =
    cam.status === "rejected"
      ? "Webcam track didn't finish. Video saved without the camera overlay."
      : undefined;
  return { folder: screen.value.folder, camWarn };
}

export function useRecordingFlow(deps: RecordingFlowDeps) {
  const { micOn, micId, displayId, sysOn, gameMode, camOn, camStream, webcam, onEdit, lastFolderRef } = deps;
  const [recording, setRecording] = useState(false);
  const [paused, setPaused] = useState(false);
  const [saving, setSaving] = useState(false);
  const [savePct, setSavePct] = useState(0);
  const [err, setErr] = useState<string | null>(null);
  const preprocessDone = useRef<(() => void) | null>(null);
  const finishing = useRef(false);
  const starting = useRef(false);
  const endEarly = useRef<() => void>(() => {});

  useEffect(() => {
    const unsubs: Promise<() => void>[] = [];
    unsubs.push(listen<number>("preprocess-progress", (e) => setSavePct(e.payload)));
    unsubs.push(listen("preprocess-done", () => preprocessDone.current?.()));
    unsubs.push(listen<string>("preprocess-error", () => preprocessDone.current?.()));
    unsubs.push(listen<string>("record-warning", (e) => setErr(e.payload)));
    unsubs.push(
      listen<string>("record-ended-early", (e) => {
        setErr(e.payload);
        endEarly.current();
      }),
    );
    return () => {
      unsubs.forEach((u) => u.then((f) => f()));
    };
  }, []);

  async function stopCore(): Promise<{ folder: string } | null> {
    const [screen, cam] = await Promise.allSettled([stopRecording(), webcam.stop()]);
    const outcome = settleStop(screen, cam);
    if ("err" in outcome) {
      setErr(outcome.err);
      return null;
    }
    if (outcome.camWarn) setErr(outcome.camWarn);
    lastFolderRef.current = outcome.folder;
    return { folder: outcome.folder };
  }

  async function finish() {
    if (finishing.current) return;
    finishing.current = true;
    setRecording(false);
    setPaused(false);
    setSaving(true);
    setSavePct(0);
    let resetSaving = true;
    try {
      const res = await stopCore();
      if (!res) return;
      await new Promise<void>((resolve) => {
        preprocessDone.current = resolve;
        preprocessProject(res.folder).catch(() => resolve());
      });
      resetSaving = await handOff(onEdit, res.folder);
    } finally {
      if (resetSaving) setSaving(false);
      preprocessDone.current = null;
      finishing.current = false;
    }
  }

  async function stopForClose(): Promise<void> {
    if (!recording && !saving) return;
    if (finishing.current) return;
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

  endEarly.current = () => {
    if (recording && !saving) void finish();
  };

  async function toggle() {
    if (saving) return;
    if (!recording) {
      if (starting.current) return;
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
    if (paused) {
      await resumeRecording();
      setPaused(false);
    } else {
      await pauseRecording();
      setPaused(true);
    }
  }

  return { recording, paused, saving, savePct, err, toggle, togglePause, stopForClose };
}
