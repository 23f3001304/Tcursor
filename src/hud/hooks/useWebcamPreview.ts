import { useCallback, useEffect, useRef, useState } from "react";

export function useWebcamPreview(deviceId: string | null, enabled: boolean) {
  const elRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const [on, setOn] = useState(false);
  const [retry, setRetry] = useState(0);

  const ref = useCallback((el: HTMLVideoElement | null) => {
    elRef.current = el;
    if (el && streamRef.current) el.srcObject = streamRef.current;
  }, []);

  useEffect(() => {
    if (!enabled) {
      setOn(false);
      return;
    }
    let cancelled = false;
    const video = deviceId ? { deviceId: { exact: deviceId } } : true;
    navigator.mediaDevices
      ?.getUserMedia({ video })
      .then((s) => {
        if (cancelled) {
          s.getTracks().forEach((t) => t.stop());
          return;
        }
        streamRef.current = s;
        if (elRef.current) elRef.current.srcObject = s;
        setOn(true);
        s.getTracks().forEach((t) => {
          t.onended = () => {
            if (!cancelled) setOn(false);
          };
        });
      })
      .catch(() => setOn(false));
    return () => {
      cancelled = true;
      streamRef.current?.getTracks().forEach((t) => t.stop());
      streamRef.current = null;
    };
  }, [deviceId, enabled, retry]);

  useEffect(() => {
    if (!enabled) return;
    const onChange = () => {
      if (!on) setRetry((r) => r + 1);
    };
    navigator.mediaDevices?.addEventListener("devicechange", onChange);
    return () => navigator.mediaDevices?.removeEventListener("devicechange", onChange);
  }, [enabled, on]);

  return { ref, stream: () => streamRef.current, on };
}
