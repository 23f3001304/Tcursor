import type { RefObject } from "react";
import { PanelHeader } from "./PanelHeader";
import { CameraMoveField } from "./CameraMoveField";
import type { AppearanceSettings } from "../../hud/settings/settings";
import type { EditDoc, EditOp } from "../../lib/edit";
import type { CamPose } from "../stage/cameraMoves";
import { DEFAULT_APPEARANCE } from "../../hud/preferences/appearanceFields";

/** What the webcam DOES over time: the Move-mode switch and the keyframes it writes.
 *
 *  What the webcam LOOKS LIKE moved out (2026-09-14). Size, shape, roundness, aspect, dock corner,
 *  margins and ring used to live here, but only for the `screen` layout - the other four layouts'
 *  webcams were unreachable from the editor at all. They are all in the Layouts panel now, one
 *  layout at a time, so there is exactly one place that answers "how does the webcam look in this
 *  layout". This panel keeps the one thing that is NOT per-layout: a camera move is a keyframed
 *  pose on the output frame, and it applies whatever layout is on screen underneath it.
 *
 *  `settings` is still read (never written) for one number: the static `cam_size` the keyframe
 *  field falls back to outside any keyframe span. */
export function CameraPanel({
  settings,
  onClose,
  doc,
  timeMs,
  applyOp,
  moveMode,
  onMoveModeChange,
  camDraftRef,
}: {
  settings: AppearanceSettings;
  onClose: () => void;
  doc: EditDoc;
  timeMs: number;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  moveMode: boolean;
  onMoveModeChange: (v: boolean) => void;
  camDraftRef: RefObject<CamPose | null>;
}) {
  // The `screen` layout's static size: what a pose falls back to where no keyframe owns the frame,
  // the same value this panel has always seeded the keyframed size slider from.
  const ma = settings?.screen ?? DEFAULT_APPEARANCE.screen;

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Camera" lede="How the webcam moves during the recording." onClose={onClose} />

      <CameraMoveField doc={doc} timeMs={timeMs} applyOp={applyOp} moveMode={moveMode}
        onMoveModeChange={onMoveModeChange} camDraftRef={camDraftRef} staticSize={ma.cam_size} />

      {/* One line, not a link: the rail is two icons away and a panel that points at another panel
          with a button would be a second navigation system. Wrapped in a group so it inherits the
          panel's 16px section rhythm rather than butting against the field above it. */}
      <div className="e-grp"><p className="e-hintline">Size, shape, position and ring live in Layouts.</p></div>
    </div>
  );
}
