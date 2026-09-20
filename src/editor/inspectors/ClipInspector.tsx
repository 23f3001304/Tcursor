import type { Clip, EditDoc, EditOp } from "../../shared/edit";
import { clipOutMs, type TimeMap } from "../../shared/math/remap";
import { Picker, Slider } from "../controls/Controls";
import { MIN_CLIP_MS } from "../hooks/input/useClipDrag";
import { Hint, InspectorHeader, InspectorShell, Section, secText, spanRange } from "./InspectorShape";
import { TimingRow } from "./InspectorRows";

export const MAX_TRANSITION_MS = 2000;

export function transitionCeiling(map: TimeMap, i: number): number {
  if (i <= 0) return 0;
  const shorter = Math.min(clipOutMs(map, i - 1), clipOutMs(map, i));
  return Math.min(MAX_TRANSITION_MS, Math.floor(shorter / 2));
}

export function ClipInspector({
  clip,
  clips,
  map,
  dur,
  motionEasing,
  onApply,
  onClose,
}: {
  clip: Clip;
  clips: Clip[];
  map: TimeMap;
  dur: number;
  motionEasing: string;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onClose: () => void;
}) {
  const i = clips.findIndex((c) => c.id === clip.id);
  const order = i + 1;
  const upd = (patch: { src_in_ms?: number; src_out_ms?: number; transition_in_ms?: number }) =>
    void onApply({ op: "update_clip", id: clip.id, ...patch });
  const outStart = clips.slice(0, i).reduce((a, _, k) => a + clipOutMs(map, k), 0);
  const ceiling = transitionCeiling(map, i);

  return (
    <InspectorShell kind="clip">
      <InspectorHeader
        title={`Clip ${order}`}
        range={spanRange(clip.src_in_ms, clip.src_out_ms)}
        deleteLabel={`Remove clip ${order}`}
        onClose={onClose}
        onDelete={() => {
          void onApply({ op: "remove_clip", id: clip.id });
          onClose();
        }}
      />

      <Section title="Source" value={secText(clip.src_out_ms - clip.src_in_ms)}>
        <TimingRow
          startMs={clip.src_in_ms}
          endMs={clip.src_out_ms}
          durMs={dur}
          minSpanMs={MIN_CLIP_MS}
          onStart={(src_in_ms) => upd({ src_in_ms })}
          onEnd={(src_out_ms) => upd({ src_out_ms })}
        />
        <Hint>The stretch of the recording this clip plays. Drag its edges on the Clips lane too.</Hint>
      </Section>

      <Section title="In the export" value={`${order} of ${clips.length}`}>
        <Hint>
          {secText(outStart)} to {secText(outStart + clipOutMs(map, i))} of the finished video.
        </Hint>
        <label className="e-field">
          <span className="e-fl">Order</span>
          <Picker
            value={String(order)}
            ariaLabel="Order"
            options={clips.map((_, k) => ({ value: String(k + 1), label: `${k + 1}` }))}
            onChange={(v) => void onApply({ op: "move_clip", id: clip.id, to_index: Number(v) - 1 })}
          />
        </label>
      </Section>

      <Section title="Transition">
        {i === 0 ? (
          <Hint>Nothing dissolves into the first clip. Move it later to give it one.</Hint>
        ) : (
          <>
            <label className="e-field">
              <Slider
                min={0}
                max={ceiling}
                step={10}
                value={Math.min(clip.transition_in_ms, ceiling)}
                onChange={(v) => upd({ transition_in_ms: Math.round(v) })}
                accentColor="var(--e-clip)"
                ariaLabel="Dissolve in"
                label="Dissolve in"
                formatValue={(v) => (v > 0 ? `${(v / 1000).toFixed(2)}s` : "Hard cut")}
              />
            </label>
            <Hint>
              The picture dissolves from the clip before this one on the {motionEasing} curve, the document's
              motion default. Two seconds at most, and never more than half the shorter clip.
            </Hint>
          </>
        )}
      </Section>
    </InspectorShell>
  );
}
