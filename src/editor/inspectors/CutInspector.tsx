import type { Cut, EditDoc, EditOp } from "../../shared/edit";
import { Hint, InspectorHeader, InspectorShell, Section, secOf, spanRange } from "./InspectorShape";
import { TimingRow } from "./InspectorRows";

export function CutInspector({
  cut,
  dur,
  onApply,
  onClose,
}: {
  cut: Cut;
  dur: number;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onClose: () => void;
}) {
  const upd = (patch: { start_ms?: number; end_ms?: number }) =>
    void onApply({ op: "update_cut", id: cut.id, ...patch });

  return (
    <InspectorShell kind="cut">
      <InspectorHeader
        title="Cut"
        range={spanRange(cut.start_ms, cut.end_ms)}
        deleteLabel="Remove cut"
        onClose={onClose}
        onDelete={() => {
          void onApply({ op: "remove_cut", id: cut.id });
          onClose();
        }}
      />

      <Section title="Timing" value={`removes ${secOf(cut.end_ms - cut.start_ms)} s`}>
        <TimingRow
          startMs={cut.start_ms}
          endMs={cut.end_ms}
          durMs={dur}
          onStart={(start_ms) => upd({ start_ms })}
          onEnd={(end_ms) => upd({ end_ms })}
        />
        <Hint>
          Everything after it moves earlier in the export. Scrub into the hatched span to see what it holds,
          the recording is still there, the export just skips it.
        </Hint>
      </Section>
    </InspectorShell>
  );
}
