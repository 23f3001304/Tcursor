import { NumberField } from "../controls/Controls";
import { CamShapeField } from "../panels/camera/CamShapeField";
import type { GraphInput } from "../motion/graphModel";
import { MotionField, motionReadout } from "../motion/MotionField";
import { KF_BLEND_MS } from "../stage/camera/cameraMoves";
import { Hint, InspectorHeader, InspectorShell, Section, secOf, secText } from "./InspectorShape";
import type { CameraMove, EditDoc, EditOp } from "../../shared/edit";

export function camGraphInput(move: CameraMove, moves: CameraMove[]): GraphInput {
  const before = moves.filter((m) => m.t_ms < move.t_ms).sort((a, b) => b.t_ms - a.t_ms)[0];
  const after = moves.filter((m) => m.t_ms > move.t_ms).sort((a, b) => a.t_ms - b.t_ms)[0];
  const startMs = before ? before.t_ms : Math.max(0, move.t_ms - KF_BLEND_MS);
  return {
    lane: "cam",
    startMs,
    endMs: move.t_ms,
    peak: 1,
    rampIn: { easing: move.easing, durMs: move.t_ms - startMs },
    rampOut: null,
    next: after ? { startMs: move.t_ms, easingIn: after.easing, durMs: after.t_ms - move.t_ms } : null,
  };
}

export function CameraMoveInspector({
  move,
  moves,
  dur,
  onApply,
  onClose,
}: {
  move: CameraMove;
  moves: CameraMove[];
  dur: number;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onClose: () => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_camera_move" }>, "op" | "id">>) =>
    void onApply({ op: "update_camera_move", id: move.id, ...patch });

  return (
    <InspectorShell kind="cam">
      <InspectorHeader
        title="Camera Move"
        range={`Keyframe at ${secText(move.t_ms)}`}
        deleteLabel="Delete keyframe"
        onClose={onClose}
        onDelete={() => {
          void onApply({ op: "remove_camera_move", id: move.id });
          onClose();
        }}
      />

      <Section title="Timing">
        <label className="e-field">
          <span className="e-fl">Time</span>
          <NumberField
            min={0}
            max={secOf(dur)}
            step={0.05}
            value={secOf(move.t_ms)}
            onChange={(v) => upd({ t_ms: Math.round(v * 1000) })}
          />
        </label>
        <Hint>Drag the diamond on the timeline to retime it.</Hint>
      </Section>

      <Section title="Placement">
        <div className="e-field2">
          <label className="e-field">
            <span className="e-fl">X</span>
            <NumberField min={0} max={1} step={0.01} unit="" value={move.x} onChange={(x) => upd({ x })} />
          </label>
          <label className="e-field">
            <span className="e-fl">Y</span>
            <NumberField min={0} max={1} step={0.01} unit="" value={move.y} onChange={(y) => upd({ y })} />
          </label>
        </div>
        <label className="e-field">
          <span className="e-fl">Size</span>
          <NumberField
            min={0}
            max={1}
            step={0.01}
            unit=""
            value={move.size}
            onChange={(size) => upd({ size })}
          />
        </label>
        <Hint>Edits preview live.</Hint>
      </Section>

      <Section title="Shape">
        <CamShapeField
          shape={move.shape}
          roundness={move.roundness}
          onShape={(shape) => upd({ shape })}
          onRoundness={(roundness) => upd({ roundness })}
        />
      </Section>

      <Section title="Motion" value={motionReadout(move.easing, null)}>
        <MotionField
          input={camGraphInput(move, moves)}
          easing={move.easing}
          easingOut={null}
          retimeable={false}
          onPatch={(p) => {
            if (p.easing !== undefined) upd({ easing: p.easing });
          }}
        />
      </Section>
    </InspectorShell>
  );
}
