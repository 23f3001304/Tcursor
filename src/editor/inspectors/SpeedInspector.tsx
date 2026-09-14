import type { EditDoc, EditOp, Speed } from "../../lib/edit";
import { FACTOR_MAX, FACTOR_MIN } from "../../lib/remap";
import { Slider } from "../controls/Controls";
import { Hint, InspectorHeader, InspectorShell, Section, TimingRow, secOf, spanRange } from "./InspectorShape";

/** The factors worth landing on exactly. A slider fine enough to reach 1.85 is also fine enough to
 *  MISS 2 - so a value inside `SNAP_WINDOW` of one of these becomes that one, and everything
 *  between stays free. Mirrors the spec's list. */
export const FACTOR_SNAPS = [0.5, 1, 1.5, 2, 4, 8];
const SNAP_WINDOW = 0.08;

/** The nearest snap within `SNAP_WINDOW`, else the raw value rounded to the slider's own step. */
export function snapFactor(v: number): number {
  const near = FACTOR_SNAPS.find((s) => Math.abs(s - v) <= SNAP_WINDOW);
  return near ?? +v.toFixed(2);
}

/** Inspector for the selected speed span: how fast it runs, and the two edges it runs between.
 *  Every control applies an `update_speed` op, so the preview retimes as the slider moves - the
 *  same live-edit contract `ZoomInspector` has. */
export function SpeedInspector({ speed, dur, onApply, onClose }: {
  speed: Speed; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}) {
  const upd = (patch: { start_ms?: number; end_ms?: number; factor?: number }) =>
    void onApply({ op: "update_speed", id: speed.id, ...patch });
  const outSec = secOf((speed.end_ms - speed.start_ms) / speed.factor);

  return (
    <InspectorShell kind="speed">
      <InspectorHeader title="Speed" range={spanRange(speed.start_ms, speed.end_ms)}
        deleteLabel="Remove speed span" onClose={onClose}
        onDelete={() => { void onApply({ op: "remove_speed", id: speed.id }); onClose(); }} />

      <Section title="Timing">
        <TimingRow startMs={speed.start_ms} endMs={speed.end_ms} durMs={dur}
          onStart={(start_ms) => upd({ start_ms })} onEnd={(end_ms) => upd({ end_ms })} />
        <Hint>Drag the pill on the Time lane to move it.</Hint>
      </Section>

      <Section title="Rate">
        <label className="e-field">
          <Slider min={FACTOR_MIN} max={FACTOR_MAX} step={0.05} value={speed.factor}
            onChange={(v) => upd({ factor: snapFactor(v) })} accentColor="var(--e-speed)" ariaLabel="Factor"
            label="Factor" formatValue={(v) => `${+v.toFixed(2)}x`} />
        </label>
        <Hint>{secOf(speed.end_ms - speed.start_ms)} s of recording becomes {outSec} s of export.</Hint>
      </Section>
    </InspectorShell>
  );
}
