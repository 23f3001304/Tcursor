import { useState } from "react";
import {
  markWebcamSegment,
  switchDisplay as switchDisplayCmd,
  switchMic as switchMicCmd,
} from "../../shared/ipc";

export async function pollFor<T>(
  get: () => T,
  ok: (v: T) => boolean,
  timeoutMs: number,
  stepMs = 100,
): Promise<T | null> {
  for (let waited = 0; waited <= timeoutMs; waited += stepMs) {
    const v = get();
    if (ok(v)) return v;
    await new Promise((r) => setTimeout(r, stepMs));
  }
  return null;
}

export const CAMERA_WAIT_MS = 5000;

export interface SourceSwitchDeps {
  recording: boolean;
  camOn: boolean;
  micOn: boolean;
  camId: string | null;
  camStream: () => MediaStream | null;
  webcam: {
    start: (stream: MediaStream | null, folder: string, segment?: number) => void;
    stop: () => Promise<void>;
    folder: () => string;
    segment: () => number;
  };
  setCamId: (id: string | null) => void;
  setMicId: (id: string) => void;
  setDisplayId: (id: string) => void;
}

export function useSourceSwitch(d: SourceSwitchDeps) {
  const [err, setErr] = useState<string | null>(null);

  async function switchCamera(id: string | null) {
    if (id !== null && id === d.camId) return;
    if (!d.recording || !d.camOn) {
      d.setCamId(id);
      return;
    }
    const prev = d.camStream()?.id ?? null;
    const dest = d.webcam.folder();
    const next = d.webcam.segment() + 1;
    await d.webcam.stop();
    d.setCamId(id);
    if (id === null || !dest) return;
    const stream = await pollFor(d.camStream, (s) => s != null && s.id !== prev, CAMERA_WAIT_MS);
    if (!stream) {
      setErr("The new camera did not start. The take is continuing without it.");
      return;
    }
    try {
      await markWebcamSegment(next);
    } catch (e) {
      setErr(`Camera switch failed: ${e}. The take is continuing without it.`);
      return;
    }
    d.webcam.start(stream, dest, next);
  }

  async function switchMic(id: string) {
    if (!d.recording || !d.micOn) {
      d.setMicId(id);
      return;
    }
    try {
      await switchMicCmd(id || null);
      d.setMicId(id);
    } catch (e) {
      setErr(`Microphone switch failed: ${e}`);
    }
  }

  async function switchDisplay(id: string) {
    if (!d.recording) {
      d.setDisplayId(id);
      return;
    }
    try {
      await switchDisplayCmd(id);
      d.setDisplayId(id);
    } catch (e) {
      setErr(`Display switch failed: ${e}`);
    }
  }

  return { err, clearErr: () => setErr(null), switchCamera, switchMic, switchDisplay };
}
