import { useRef } from "react";
import { appendWebcam } from "../../shared/ipc";

export type WebcamStopAction = "skip" | "stop";
export function webcamStopAction(state: RecordingState | null): WebcamStopAction {
  return state === null || state === "inactive" ? "skip" : "stop";
}

export function raceStopOrTimeout(onDone: Promise<void>, timeoutMs: number): Promise<"stopped" | "timeout"> {
  return Promise.race([
    onDone.then((): "stopped" => "stopped"),
    new Promise<"timeout">((res) => setTimeout(() => res("timeout"), timeoutMs)),
  ]);
}

export function useWebcamRecorder() {
  const rec = useRef<MediaRecorder | null>(null);
  const chain = useRef<Promise<void>>(Promise.resolve());
  const folder = useRef<string>("");
  const segment = useRef(1);

  function start(stream: MediaStream | null, dest: string, seg = 1) {
    if (!stream) return;
    folder.current = dest;
    segment.current = seg;
    chain.current = Promise.resolve();
    const mr = new MediaRecorder(stream, { mimeType: "video/webm" });
    mr.ondataavailable = (e) => {
      if (!e.data.size) return;
      const f = folder.current,
        s = segment.current;
      chain.current = chain.current
        .then(() => e.data.arrayBuffer())
        .then((buf) => appendWebcam(f, new Uint8Array(buf), s))
        .catch(() => {});
    };
    mr.start(1000);
    rec.current = mr;
  }

  async function stop(): Promise<void> {
    const mr = rec.current;
    rec.current = null;
    if (webcamStopAction(mr?.state ?? null) === "skip") {
      await chain.current;
      return;
    }
    const mrLive = mr as MediaRecorder;
    const stopped = new Promise<void>((res) => {
      mrLive.onstop = () => res();
    });
    try {
      mrLive.stop();
    } catch {
      await chain.current;
      return;
    }
    await raceStopOrTimeout(stopped, 4000);
    await chain.current;
  }

  return { start, stop, folder: () => folder.current, segment: () => segment.current };
}
