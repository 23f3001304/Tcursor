import { springOf } from "../../shared/math/spring";
import { SpringControls } from "../inspectors/SpringControls";
import type { GraphInput } from "./graphModel";
import { MotionGraph } from "./MotionGraph";
import { PRESETS, presetOf, presetPatch } from "./presets";
import { CUSTOM, PresetRow } from "./PresetRow";
import type { GraphPatch } from "./graphEdits";

export function MotionField({
  input,
  easing,
  easingOut,
  retimeable = true,
  onPatch,
}: {
  input: GraphInput;
  easing: string;
  easingOut?: string | null;
  retimeable?: boolean;
  onPatch: (patch: GraphPatch) => void;
}) {
  const id = presetOf(easing, easingOut);
  const spr = springOf(easing);
  return (
    <>
      <PresetRow
        presets={PRESETS}
        value={id}
        onPick={(k) => {
          const p = presetPatch(k);
          onPatch({ easing: p.easing, easing_out: p.easing_out });
        }}
      />
      <MotionGraph input={input} onCommit={onPatch} retimeable={retimeable} />
      {spr && (
        <SpringControls
          stiffness={spr[0]}
          damping={spr[1]}
          mass={spr[2]}
          onChange={(e) => onPatch({ easing: e, easing_out: e })}
        />
      )}
    </>
  );
}

export function motionReadout(easing: string, easingOut?: string | null): string | undefined {
  return presetOf(easing, easingOut) === CUSTOM ? "Custom" : undefined;
}
