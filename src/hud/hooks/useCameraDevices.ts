import { useEffect, useState } from "react";

export interface CameraInfo {
  id: string;
  label: string;
}

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
