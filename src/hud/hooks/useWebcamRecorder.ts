import { useRef } from "react";

export function useWebcamRecorder() {
  const rec = useRef<MediaRecorder | null>(null);
  const chunks = useRef<Blob[]>([]);

  function start(stream: MediaStream | null) {
    if (!stream) return;
    chunks.current = [];
    const mr = new MediaRecorder(stream, { mimeType: "video/webm" });
    mr.ondataavailable = (e) => { if (e.data.size) chunks.current.push(e.data); };
    mr.start();
    rec.current = mr;
  }

  async function stop(): Promise<Uint8Array | null> {
    const mr = rec.current;
    if (!mr) return null;
    const done = new Promise<Blob>((res) => { mr.onstop = () => res(new Blob(chunks.current, { type: "video/webm" })); });
    mr.stop();
    rec.current = null;
    const buf = await (await done).arrayBuffer();
    return new Uint8Array(buf);
  }

  return { start, stop };
}
