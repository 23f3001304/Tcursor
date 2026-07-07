import { useRef } from "react";
import { appendWebcam } from "../../lib/ipc";

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

  async function stop(): Promise<void> {
    const mr = rec.current;
    if (!mr) return;
    const stopped = new Promise<void>((res) => { mr.onstop = () => res(); });
    mr.stop(); // fires a final ondataavailable (queues its append), then onstop
    rec.current = null;
    await stopped;
    await chain.current; // all chunks flushed to disk
  }

  return { start, stop };
}
