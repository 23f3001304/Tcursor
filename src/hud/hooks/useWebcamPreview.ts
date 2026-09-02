import { useCallback, useEffect, useRef, useState } from "react";

/** Webcam preview for the chosen device; runs only while `enabled`. Preview only.
 *  The live stream is kept in a ref and re-attached through a callback ref every
 *  time the <video> mounts, so the preview survives the element remounting when the
 *  settings box opens/closes (otherwise the new <video> has no srcObject -> black). */
export function useWebcamPreview(deviceId: string | null, enabled: boolean) {
  const elRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const [on, setOn] = useState(false);
  // Bumped by the devicechange listener below to force a retry once `on` is stuck false while
  // still `enabled` - re-plugging a camera changes neither `deviceId` nor `enabled`, so without
  // this the acquire effect below never re-runs and the preview stays dead until app restart
  // (M4).
  const [retry, setRetry] = useState(0);

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
        // A track ending (unplug, driver reset, another app taking exclusive access) is the only
        // way the browser tells us the device died mid-preview - reflect it immediately (M4) so
        // the HUD's camera thumb drops to its off/no-device glyph instead of freezing on the last
        // frame while still claiming "on".
        s.getTracks().forEach((t) => { t.onended = () => { if (!cancelled) setOn(false); }; });
      })
      .catch(() => setOn(false));
    return () => {
      cancelled = true;
      streamRef.current?.getTracks().forEach((t) => t.stop());
      streamRef.current = null;
    };
  }, [deviceId, enabled, retry]);

  // Re-plugging fires `devicechange` but changes neither dependency above, so this is the only
  // thing that lets the preview recover on its own. Only retries while we're actually supposed
  // to be showing a preview and currently aren't.
  useEffect(() => {
    if (!enabled) return;
    const onChange = () => { if (!on) setRetry((r) => r + 1); };
    navigator.mediaDevices?.addEventListener("devicechange", onChange);
    return () => navigator.mediaDevices?.removeEventListener("devicechange", onChange);
  }, [enabled, on]);

  return { ref, stream: () => streamRef.current, on };
}
