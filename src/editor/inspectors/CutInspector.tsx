import type { Cut, EditDoc, EditOp } from "../../lib/edit";
import { Hint, InspectorHeader, InspectorShell, Section, TimingRow, secOf, spanRange } from "./InspectorShape";

/** Inspector for the selected cut: the two edges, in clip seconds, and Remove. Numeric only - a cut
 *  is a range of removed time, not a thing with a look, and its hatched span on the timeline has no
 *  handles of its own (a millisecond is reachable here, not by dragging a 2px edge).
 *
 *  Start/End are CLIP time, the same clock every pill on the timeline sits on, so a cut's numbers
 *  keep meaning the same thing after another cut lands earlier in the clip. */
export function CutInspector({ cut, dur, onApply, onClose }: {
  cut: Cut; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}) {
  const upd = (patch: { start_ms?: number; end_ms?: number }) =>
    void onApply({ op: "update_cut", id: cut.id, ...patch });

  return (
    <InspectorShell kind="cut">
      <InspectorHeader title="Cut" range={spanRange(cut.start_ms, cut.end_ms)}
        deleteLabel="Remove cut" onClose={onClose}
        onDelete={() => { void onApply({ op: "remove_cut", id: cut.id }); onClose(); }} />

      <Section title="Timing" value={`removes ${secOf(cut.end_ms - cut.start_ms)} s`}>
        <TimingRow startMs={cut.start_ms} endMs={cut.end_ms} durMs={dur}
          onStart={(start_ms) => upd({ start_ms })} onEnd={(end_ms) => upd({ end_ms })} />
        <Hint>Everything after it moves earlier in the export. Scrub into the hatched span to see what it holds, the recording is still there, the export just skips it.</Hint>
      </Section>
    </InspectorShell>
  );
}
