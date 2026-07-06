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

/** One overridable spotlight param: field name on the left, the current value (or "Default
 *  (X%)" when unset) grouped right next to its enable/disable switch on the right - so the
 *  value description reads next to the control it describes, not crammed against the label. */
function OverrideField({ label, value, defaultValue, min, max, step, onToggle, onChange }: {
  label: string; value: number | undefined; defaultValue: number;
  min: number; max: number; step: number;
  onToggle: (on: boolean) => void; onChange: (v: number) => void;
}) {
  return (
    <label className="e-field">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
        <span className="e-fl" style={{ margin: 0 }}>{label}</span>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontSize: 12, color: "var(--e-mut)" }}>
            {value !== undefined
              ? <b style={{ color: "var(--e-fg)" }}>{Math.round(value * 100)}%</b>
              : <>Default <b style={{ color: "var(--e-fg)" }}>({Math.round(defaultValue * 100)}%)</b></>}
          </span>
          <Switch on={value !== undefined} onChange={onToggle} />
        </div>
      </div>
      <Slider min={min} max={max} step={step} value={value ?? defaultValue} disabled={value === undefined}
        onChange={onChange} accentColor="var(--e-fx)" />
    </label>
  );
}

/** Inspector for the selected effect region (spotlight). v1 edits start/end + delete; the
 *  spotlight's look comes from Settings (per-region params are a later addition). Shown in the
 *  left panel in place of the tab content while an effect region is selected. */
export function EffectInspector({ effect, dur, settings, onApply, onClose }: {
  effect: EffectRegion; dur: number; settings: Settings; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
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
        <Picker value={effect.mode || "global"} options={MODE_OPTIONS} onChange={(v) => upd({ mode: v })} />
      </label>

      <OverrideField label="Dim" value={effect.dim} defaultValue={defaultDim} min={0.2} max={0.9} step={0.05}
        onToggle={(on) => upd({ dim: on ? defaultDim : -1 })} onChange={(v) => upd({ dim: v })} />

      <OverrideField label="Radius" value={effect.radius} defaultValue={defaultRadius} min={0.05} max={0.30} step={0.01}
        onToggle={(on) => upd({ radius: on ? defaultRadius : -1 })} onChange={(v) => upd({ radius: v })} />

      <OverrideField label="Feather" value={effect.feather} defaultValue={defaultFeather} min={0.02} max={0.25} step={0.01}
        onToggle={(on) => upd({ feather: on ? defaultFeather : -1 })} onChange={(v) => upd({ feather: v })} />

      <button className="e-del" onClick={() => { void onApply({ op: "remove_effect", id: effect.id }); onClose(); }}>
        <IconTrash size={15} /> Delete spotlight
      </button>
    </div>
  );
}
