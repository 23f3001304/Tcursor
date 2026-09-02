import type { RefCallback } from "react";
import { Camera } from "./icons";

/** The recording bar's live camera-preview tile. Split out of `Hud.tsx` (200-line cap) so
 *  gate-feedback item 4's saving-state handling (user-reported 2026-09-02: the live webcam
 *  preview kept showing, and the camera stayed live, in the saving bar) has a single, obvious
 *  home instead of another inline conditional in an already-dense render.
 *
 *  Owns no state - purely a rendering of the truth its props already carry. `Hud.tsx` renders this
 *  ONLY while `!saving` (hides the tile entirely); the actual camera LED going off is
 *  `useWebcamPreview`'s own `enabled` gate (`Hud.tsx` passes `camOn && !saving`) releasing the
 *  underlying `MediaStream` - this component just stops SHOWING a tile once that stream is gone,
 *  it never owns the stream itself. */
export function CamTile({ camRef, camOn, camLive }: { camRef: RefCallback<HTMLVideoElement>; camOn: boolean; camLive: boolean }) {
  return (
    <div className="camtoggle" title="Camera preview">
      <video ref={camRef} className={`cam ${camOn && camLive ? "" : "off"}`} autoPlay muted playsInline />
      {!(camOn && camLive) && <span className="camoff"><Camera /></span>}
    </div>
  );
}
