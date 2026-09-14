import type { EditDoc, EditOp, EffectRegion } from "../../lib/edit";
import type { Settings } from "../../hud/settings/settings";
import { Switch, Picker, NumberField, Slider } from "../controls/Controls";
import { Hint, InspectorHeader, InspectorShell, Section, TimingRow, secOf, spanRange } from "./InspectorShape";

const MODE_OPTIONS = [
  { value: "global", label: "Use Global Default" },
  { value: "classic", label: "Classic" },
  { value: "blur", label: "Blur" },
  { value: "halo", label: "Halo" },
  { value: "breathing", label: "Breathing" },
  { value: "nebula", label: "Nebula" },
  { value: "vignette", label: "Vignette" },
];

/** One overridable spotlight param: the Override switch directly above the slider it enables, and
 *  the Slider's own live readout ("Dim 65%", or "Dim Default (90%)" while off) on the label row.
 *  The switch row says "Override", not `label` again - the slider's row already prints the field
 *  name once. While NOT overridden the slider reads back its own `disabled` styling and refuses
 *  pointer/keyboard input. The outer wrapper is a plain `<div>`, not a `<label>`: a `<label>`
 *  forwards any click inside it to its first labelable control, so releasing a Slider drag also
 *  fired the Switch and silently wiped the override. Both carry their own `aria-label`. */
function OverrideField({ label, value, defaultValue, min, max, step, onToggle, onChange }: {
  label: string; value: number | undefined; defaultValue: number;
  min: number; max: number; step: number;
  onToggle: (on: boolean) => void; onChange: (v: number) => void;
}) {
  const overridden = value !== undefined;
  return (
    <div className="e-field">
      <div className="e-switchrow" style={{ marginBottom: 4 }}>
        <span className="e-fl">Override</span>
        <Switch on={overridden} onChange={onToggle} ariaLabel={`Override ${label}`} />
      </div>
      <Slider min={min} max={max} step={step} value={value ?? defaultValue} disabled={!overridden}
        onChange={onChange} accentColor="var(--e-fx)" ariaLabel={label}
        label={label} formatValue={(v) => overridden ? `${Math.round(v * 100)}%` : `Default (${Math.round(v * 100)}%)`} />
    </div>
  );
}

/** Inspector for the selected effect region (spotlight): when it runs, how it looks, how it fades
 *  in and out. The per-region dim/radius/feather each override the global Settings default. */
export function EffectInspector({ effect, dur, settings, onApply, onDimCamera, onClose }: {
  effect: EffectRegion; dur: number; settings: Settings; onApply: (op: EditOp) => Promise<EditDoc | null>;
  onDimCamera: (v: boolean) => void; onClose: () => void;
}) {
  const upd = (patch: { start_ms?: number; end_ms?: number; fade_in_ms?: number; fade_out_ms?: number; mode?: string; dim?: number; radius?: number; feather?: number }) =>
    void onApply({ op: "update_effect", id: effect.id, ...patch });
  const span = secOf(effect.end_ms - effect.start_ms);
  const d = settings.clickfx;

  return (
    <InspectorShell kind="fx">
      <InspectorHeader title="Spotlight" range={spanRange(effect.start_ms, effect.end_ms)}
        deleteLabel="Delete spotlight" onClose={onClose}
        onDelete={() => { void onApply({ op: "remove_effect", id: effect.id }); onClose(); }} />

      <Section title="Timing">
        <TimingRow startMs={effect.start_ms} endMs={effect.end_ms} durMs={dur}
          onStart={(start_ms) => upd({ start_ms })} onEnd={(end_ms) => upd({ end_ms })} />
        <Hint>Dims everything but the cursor for this span. Drag the block on the timeline to move it.</Hint>
      </Section>

      <Section title="Look">
        <label className="e-field">
          <span className="e-fl">Spotlight Mode</span>
          <Picker value={effect.mode || "global"} options={MODE_OPTIONS} onChange={(mode) => upd({ mode })} ariaLabel="Spotlight Mode" />
        </label>
        <OverrideField label="Dim" value={effect.dim} defaultValue={d.spotlight_dim} min={0.2} max={0.9} step={0.05}
          onToggle={(on) => upd({ dim: on ? d.spotlight_dim : -1 })} onChange={(dim) => upd({ dim })} />
        <OverrideField label="Radius" value={effect.radius} defaultValue={d.spotlight_radius} min={0.05} max={0.30} step={0.01}
          onToggle={(on) => upd({ radius: on ? d.spotlight_radius : -1 })} onChange={(radius) => upd({ radius })} />
        <OverrideField label="Feather" value={effect.feather} defaultValue={d.spotlight_feather} min={0.02} max={0.25} step={0.01}
          onToggle={(on) => upd({ feather: on ? d.spotlight_feather : -1 })} onChange={(feather) => upd({ feather })} />
        <div className="e-switchrow">
          <span>Dim webcam</span>
          <Switch on={d.spotlight_dim_camera} onChange={onDimCamera} ariaLabel="Dim webcam" />
        </div>
        <Hint>Off keeps the webcam PiP lit while the spotlight dims everything else. Applies to all spotlights.</Hint>
      </Section>

      <Section title="Fades">
        <div className="e-field2">
          <label className="e-field"><span className="e-fl">Fade in</span>
            <NumberField step={0.05} min={0} max={span} value={secOf(effect.fade_in_ms)}
              onChange={(v) => upd({ fade_in_ms: Math.round(v * 1000) })} /></label>
          <label className="e-field"><span className="e-fl">Fade out</span>
            <NumberField step={0.05} min={0} max={span} value={secOf(effect.fade_out_ms)}
              onChange={(v) => upd({ fade_out_ms: Math.round(v * 1000) })} /></label>
        </div>
      </Section>
    </InspectorShell>
  );
}
