import { useCallback, useEffect, useRef, useState } from "react";

/** Webcam preview for the chosen device; runs only while `enabled`. Preview only.
 *  The live stream is kept in a ref and re-attached through a callback ref every
 *  time the <video> mounts, so the preview survives the element remounting when the
 *  settings box opens/closes (otherwise the new <video> has no srcObject -> black). */
export function useWebcamPreview(deviceId: string | null, enabled: boolean) {
  const elRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const [on, setOn] = useState(false);

  const ref = useCallback((el: HTMLVideoElement | null) => {
    elRef.current = el;
    if (el && streamRef.current) el.srcObject = streamRef.current;
  }, []);

  useEffect(() => {
    if (!enabled) { setOn(false); return; }
    let cancelled = false;
    const video = deviceId ? { deviceId: { exact: deviceId } } : true;
    navigator.mediaDevices
      ?.getUserMedia({ video })
      .then((s) => {
        if (cancelled) { s.getTracks().forEach((t) => t.stop()); return; }
        streamRef.current = s;
        if (elRef.current) elRef.current.srcObject = s;
        setOn(true);
      })
      .catch(() => setOn(false));
    return () => {
      cancelled = true;
      streamRef.current?.getTracks().forEach((t) => t.stop());
      streamRef.current = null;
    };
  }, [deviceId, enabled]);

  return { ref, stream: () => streamRef.current, on };
}
