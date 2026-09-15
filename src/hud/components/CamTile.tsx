import type { RefCallback } from "react";
import { Camera } from "./icons";

export function CamTile({
  camRef,
  camOn,
  camLive,
  shape,
}: {
  camRef: RefCallback<HTMLVideoElement>;
  camOn: boolean;
  camLive: boolean;
  shape?: "wide" | "round";
}) {
  return (
    <div className={`camtoggle${shape ? ` ${shape}` : ""}`} title="Camera preview">
      <video ref={camRef} className={`cam ${camOn && camLive ? "" : "off"}`} autoPlay muted playsInline />
      {!(camOn && camLive) && (
        <span className="camoff">
          <Camera />
        </span>
      )}
    </div>
  );
}
