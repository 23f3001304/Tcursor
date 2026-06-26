import { useEffect, useRef, useState } from "react";

/** Webcam preview for the chosen device; runs only while `enabled`. Preview only. */
export function useWebcamPreview(deviceId: string | null, enabled: boolean) {
  const ref = useRef<HTMLVideoElement>(null);
  const [on, setOn] = useState(false);
  useEffect(() => {
    if (!enabled) { setOn(false); return; }
    let stream: MediaStream | null = null;
    const video = deviceId ? { deviceId: { exact: deviceId } } : true;
    navigator.mediaDevices
      ?.getUserMedia({ video })
      .then((s) => {
        stream = s;
        if (ref.current) {
          ref.current.srcObject = s;
          setOn(true);
        }
      })
      .catch(() => setOn(false));
    return () => stream?.getTracks().forEach((t) => t.stop());
  }, [deviceId, enabled]);
  return { ref, on };
}
