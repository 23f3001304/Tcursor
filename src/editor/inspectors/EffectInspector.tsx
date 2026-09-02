import { IconTrash } from "@tabler/icons-react";
import { PanelHeader } from "../panels/PanelHeader";
import type { EditDoc, EditOp, EffectRegion } from "../../lib/edit";
import type { Settings } from "../../hud/settings/settings";
import { Switch, Picker, NumberField, Slider } from "../controls/Controls";

const MODE_OPTIONS = [
  { value: "global", label: "Use Global Default" },
  { value: "classic", label: "Classic" },
  { value: "blur", label: "Blur" },
  { value: "halo", label: "Halo" },
  { value: "breathing", label: "Breathing" },
  { value: "nebula", label: "Nebula" },
  { value: "vignette", label: "Vignette" },
];

/** One overridable spotlight param: an enable/disable switch on its own row, then the Slider's
 *  own `label`/`formatValue` readout ("Dim 65%" - or "Dim Default (90%)" while off) sourced from
 *  the LIVE value, not the committed prop (fix round 2 - this used to render a static `<b>` next
 *  to the switch instead, which never moved during a drag). The switchrow says "Override", not
 *  `label` again - the Slider's own row already prints the field name once (fix round 3: this
 *  used to print `label` in BOTH rows, e.g. "Dim" over "Dim 65%"); the Switch keeps the real name
 *  via `aria-label` so it's still unambiguous non-visually. While NOT overridden, the slider
 *  reads back its own `disabled` styling (opacity halved, thumb dims, no pointer/keyboard
 *  interaction - see Slider.tsx) - the fix here is real disablement, not just the ambiguous
 *  always-interactive look ux audit #22 flagged. The outer wrapper is a plain `<div>`, not a
 *  `<label>` (fix round 4): a `<label>` forwards any click inside it to its first labelable
 *  control, so releasing a Slider drag also fired the Switch's click handler and silently wiped
 *  the override. Both `Switch` and `Slider` carry their own `aria-label`, so no label semantics
 *  are lost. */
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

/** Inspector for the selected effect region (spotlight). v1 edits start/end + delete; the
 *  spotlight's look comes from Settings (per-region params are a later addition). Shown in the
 *  left panel in place of the tab content while an effect region is selected. */
export function EffectInspector({ effect, dur, settings, onApply, onDimCamera, onClose }: {
  effect: EffectRegion; dur: number; settings: Settings; onApply: (op: EditOp) => Promise<EditDoc | null>;
  onDimCamera: (v: boolean) => void; onClose: () => void;
}) {
  const sec = (ms: number) => +(ms / 1000).toFixed(2);
  const upd = (patch: { start_ms?: number; end_ms?: number; fade_in_ms?: number; fade_out_ms?: number; mode?: string; dim?: number; radius?: number; feather?: number }) =>
    void onApply({ op: "update_effect", id: effect.id, ...patch });

  const defaultDim = settings.clickfx.spotlight_dim;
  const defaultRadius = settings.clickfx.spotlight_radius;
  const defaultFeather = settings.clickfx.spotlight_feather;

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Spotlight" lede="Dims everything but the cursor for this span. Drag the block on the timeline to move it." closeTitle="Deselect" onClose={onClose} />

      <div className="e-field2">
        <label className="e-field"><span className="e-fl">Start</span>
          <NumberField min={0} max={sec(effect.end_ms)} value={sec(effect.start_ms)}
            onChange={(v) => upd({ start_ms: Math.round(v * 1000) })} /></label>
        <label className="e-field"><span className="e-fl">End</span>
          <NumberField min={sec(effect.start_ms)} max={sec(dur)} value={sec(effect.end_ms)}
            onChange={(v) => upd({ end_ms: Math.round(v * 1000) })} /></label>
      </div>

      <div className="e-field2">
        <label className="e-field"><span className="e-fl">Fade in</span>
          <NumberField step={0.05} min={0} max={sec(effect.end_ms - effect.start_ms)} value={sec(effect.fade_in_ms)}
            onChange={(v) => upd({ fade_in_ms: Math.round(v * 1000) })} /></label>
        <label className="e-field"><span className="e-fl">Fade out</span>
          <NumberField step={0.05} min={0} max={sec(effect.end_ms - effect.start_ms)} value={sec(effect.fade_out_ms)}
            onChange={(v) => upd({ fade_out_ms: Math.round(v * 1000) })} /></label>
      </div>

      <label className="e-field">
        <span className="e-fl">Spotlight Mode</span>
        <Picker value={effect.mode || "global"} options={MODE_OPTIONS} onChange={(v) => upd({ mode: v })} ariaLabel="Spotlight Mode" />
      </label>

      <OverrideField label="Dim" value={effect.dim} defaultValue={defaultDim} min={0.2} max={0.9} step={0.05}
        onToggle={(on) => upd({ dim: on ? defaultDim : -1 })} onChange={(v) => upd({ dim: v })} />

      <OverrideField label="Radius" value={effect.radius} defaultValue={defaultRadius} min={0.05} max={0.30} step={0.01}
        onToggle={(on) => upd({ radius: on ? defaultRadius : -1 })} onChange={(v) => upd({ radius: v })} />

      <OverrideField label="Feather" value={effect.feather} defaultValue={defaultFeather} min={0.02} max={0.25} step={0.01}
        onToggle={(on) => upd({ feather: on ? defaultFeather : -1 })} onChange={(v) => upd({ feather: v })} />

      <div className="e-field" style={{ marginTop: 4 }}>
        <div className="e-switchrow">
          <span>Dim webcam</span>
          <Switch on={settings.clickfx.spotlight_dim_camera} onChange={onDimCamera} />
        </div>
        <span className="e-lede" style={{ marginTop: 4 }}>Off keeps the webcam PiP lit while the spotlight dims everything else. Applies to all spotlights.</span>
      </div>

      <button className="e-del" onClick={() => { void onApply({ op: "remove_effect", id: effect.id }); onClose(); }}>
        <IconTrash size={15} />Delete spotlight
      </button>
    </div>
  );
}
