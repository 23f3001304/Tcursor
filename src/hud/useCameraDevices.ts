import { useEffect, useState } from "react";

export interface CameraInfo { id: string; label: string }

/** Enumerate video input devices. Labels populate once camera permission is granted,
 *  so `refreshKey` lets the caller re-enumerate after the preview turns on. */
export function useCameraDevices(refreshKey: number) {
  const [cameras, setCameras] = useState<CameraInfo[]>([]);
  useEffect(() => {
    navigator.mediaDevices
      ?.enumerateDevices()
      .then((ds) =>
        setCameras(
          ds
            .filter((d) => d.kind === "videoinput")
            .map((d, i) => ({ id: d.deviceId, label: d.label || `Camera ${i + 1}` })),
        ),
      )
      .catch(() => {});
  }, [refreshKey]);
  return cameras;
}
