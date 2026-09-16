import type { EditDoc, EditOp, TextAnchor, TextAnim, TextItem, TextSize } from "../../shared/edit";
import { TextContentField } from "./TextContentField";
import { NumberField, Picker, Segmented, Slider } from "../controls/Controls";
import { MotionField, motionReadout } from "../motion/MotionField";
import { TEXT_KIND_OPTIONS, TEXT_STYLE_OPTIONS } from "../panels/textStyles";
import { Hint, InspectorHeader, InspectorShell, Section, secOf, secText, spanRange } from "./InspectorShape";
import { SegRow, TimingRow, type SegOption } from "./InspectorRows";
import {
  ANCHOR_GRID,
  ANIM_OPTIONS,
  SIZE_OPTIONS,
  slideIsPointless,
  textGraphInput,
} from "./textInspectorModel";

type Patch = Partial<Omit<TextItem, "id">>;

export function TextInspector({
  item,
  dur,
  onApply,
  onClose,
}: {
  item: TextItem;
  dur: number;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onClose: () => void;
}) {
  const upd = (patch: Patch) => void onApply({ op: "update_text", id: item.id, ...patch });
  const label = TEXT_KIND_OPTIONS.find((k) => k.value === item.kind)?.label ?? "Text";
  const noSlide = slideIsPointless(item.pos);
  const anims = ANIM_OPTIONS.map((o) => ({
    value: o.value as TextAnim,
    label: o.label,
    title: o.value === "slide" && noSlide ? "A centred item has no edge to slide from" : undefined,
  }));

  const anchors: SegOption[] = ANCHOR_GRID.map((a) => ({
    key: a,
    label: "",
    on: item.pos === a,
    title: a.replace("_", " "),
    visual: <span className="e-anchordot" aria-hidden="true" />,
  }));

  return (
    <InspectorShell kind="caption">
      <InspectorHeader
        title={label}
        range={spanRange(item.start_ms, item.end_ms)}
        deleteLabel={`Delete ${label.toLowerCase()}`}
        onClose={onClose}
        onDelete={() => {
          void onApply({ op: "remove_text", id: item.id });
          onClose();
        }}
      />

      <Section title="Timing" value={secText(item.end_ms - item.start_ms)}>
        <TimingRow
          startMs={item.start_ms}
          endMs={item.end_ms}
          durMs={dur}
          onStart={(start_ms) => upd({ start_ms })}
          onEnd={(end_ms) => upd({ end_ms })}
        />
        <Hint>Drag the pill on the Text lane to move it.</Hint>
      </Section>

      <Section title="Content">
        <TextContentField
          item={item}
          onText={(text) => upd({ text })}
          onSub={(sub) => upd({ sub })}
          onKind={(kind) => upd({ kind })}
        />
      </Section>

      <Section title="Look" value={TEXT_STYLE_OPTIONS.find((s) => s.value === item.style)?.label}>
        <div className="e-field">
          <span className="e-fl">Style</span>
          <Picker
            value={item.style}
            options={TEXT_STYLE_OPTIONS}
            onChange={(style) => upd({ style })}
            ariaLabel="Style"
          />
        </div>
        <div className="e-anchorgrid">
          <SegRow
            options={anchors}
            onPick={(key) => upd({ pos: key as TextAnchor })}
            ariaLabel="Where it sits in the frame"
          />
        </div>
        <Slider
          min={-50}
          max={50}
          step={1}
          value={Math.round(item.offset[0] * 100)}
          onChange={(v) => upd({ offset: [v / 100, item.offset[1]] })}
          ariaLabel="Nudge X"
          label="Nudge X"
          formatValue={(v) => `${v}%`}
        />
        <Slider
          min={-50}
          max={50}
          step={1}
          value={Math.round(item.offset[1] * 100)}
          onChange={(v) => upd({ offset: [item.offset[0], v / 100] })}
          ariaLabel="Nudge Y"
          label="Nudge Y"
          formatValue={(v) => `${v}%`}
        />
        <div className="e-field">
          <span className="e-fl">Size</span>
          <Segmented
            value={item.size}
            options={SIZE_OPTIONS.map((o) => ({ value: o.value as TextSize, label: o.label }))}
            onChange={(size) => upd({ size })}
            ariaLabel="Size"
          />
        </div>
      </Section>

      <Section title="Motion" value={motionReadout(item.easing)}>
        <div className="e-field">
          <span className="e-fl">In</span>
          <Picker
            value={item.anim_in}
            options={anims}
            onChange={(anim_in) => upd({ anim_in })}
            ariaLabel="In"
          />
          <NumberField
            value={secOf(item.in_ms)}
            min={0}
            max={10}
            onChange={(v) => upd({ in_ms: Math.round(v * 1000) })}
          />
        </div>
        <div className="e-field">
          <span className="e-fl">Out</span>
          <Picker
            value={item.anim_out}
            options={anims}
            onChange={(anim_out) => upd({ anim_out })}
            ariaLabel="Out"
          />
          <NumberField
            value={secOf(item.out_ms)}
            min={0}
            max={10}
            onChange={(v) => upd({ out_ms: Math.round(v * 1000) })}
          />
        </div>
        <MotionField
          input={textGraphInput(item)}
          easing={item.easing}
          onPatch={(p) => upd({ easing: p.easing, in_ms: p.inMs, out_ms: p.outMs })}
        />
        {item.anim_in === "typewriter" && <Hint>The whole line is revealed over the In duration.</Hint>}
        {noSlide && <Hint>Slide has no edge to come from at the centre, so it reads as a fade.</Hint>}
        <Hint>Captions are drawn last and nothing covers them, so nudge a line clear of one.</Hint>
      </Section>
    </InspectorShell>
  );
}
