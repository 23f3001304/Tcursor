import type { EditDoc, EditOp, EffectRegion } from "../../shared/edit";
import type { Settings } from "../../hud/settings/settings";
import { NumberField, Slider } from "../controls/Controls";
import { Hint, InspectorHeader, InspectorShell, Section, secOf, spanRange } from "./InspectorShape";
import { TimingRow } from "./InspectorRows";

const NAMES: Record<string, string> = { blur: "Blur", pixelate: "Pixelate", highlight: "Highlight" };

const DEFAULTS = { blur: 0.02, pixelate: 0.018, roundness: 0.06, feather: 0.01 };

type Patch = Partial<Omit<Extract<EditOp, { op: "update_effect" }>, "op" | "id">>;

export function MaskInspector({
  effect,
  dur,
  settings,
  onApply,
  onClose,
}: {
  effect: EffectRegion;
  dur: number;
  settings: Settings;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onClose: () => void;
}) {
  const upd = (patch: Patch) => void onApply({ op: "update_effect", id: effect.id, ...patch });
  const name = NAMES[effect.kind] ?? "Mask";
  const span = secOf(effect.end_ms - effect.start_ms);
  const rect = effect.rect ?? [0.35, 0.4, 0.3, 0.2];
  const setRect = (i: number, pct: number) => {
    const next = [...rect] as [number, number, number, number];
    next[i] = pct / 100;
    upd({ rect: next });
  };
  const pixels = effect.kind === "pixelate";
  const amount = effect.strength ?? (pixels ? DEFAULTS.pixelate : DEFAULTS.blur);

  return (
    <InspectorShell kind="fx">
      <InspectorHeader
        title={name}
        range={spanRange(effect.start_ms, effect.end_ms)}
        deleteLabel={`Delete ${name.toLowerCase()}`}
        onClose={onClose}
        onDelete={() => {
          void onApply({ op: "remove_effect", id: effect.id });
          onClose();
        }}
      />

      <Section title="Timing">
        <TimingRow
          startMs={effect.start_ms}
          endMs={effect.end_ms}
          durMs={dur}
          onStart={(start_ms) => upd({ start_ms })}
          onEnd={(end_ms) => upd({ end_ms })}
        />
      </Section>

      <Section title="Region">
        <div className="e-field2 e-maskregion">
          {(["X", "Y", "Width", "Height"] as const).map((label, i) => (
            <label className="e-field" key={label}>
              <span className="e-fl">{label}</span>
              <NumberField
                step={1}
                min={i < 2 ? 0 : 1}
                max={100}
                value={Math.round(rect[i] * 100)}
                onChange={(v) => setRect(i, v)}
              />
            </label>
          ))}
        </div>
        <Hint>
          Drag the box on the stage. The region follows the picture through zooms and layout changes.
        </Hint>
      </Section>

      <Section title="Look">
        {effect.kind === "highlight" ? (
          <div className="e-field">
            <Slider
              min={20}
              max={90}
              step={1}
              value={Math.round((effect.dim ?? settings.clickfx.spotlight_dim) * 100)}
              onChange={(v) => upd({ dim: v / 100 })}
              accentColor="var(--e-fx)"
              ariaLabel="Dim"
              label="Dim"
              formatValue={(v) => `${Math.round(v)}%`}
            />
          </div>
        ) : (
          <div className="e-field">
            <Slider
              min={0.2}
              max={12}
              step={0.1}
              value={+(amount * 100).toFixed(1)}
              onChange={(v) => upd({ strength: v / 100 })}
              accentColor="var(--e-fx)"
              ariaLabel={pixels ? "Pixel size" : "Blur radius"}
              label={pixels ? "Pixel size" : "Blur radius"}
              formatValue={(v) => `${v.toFixed(1)}%`}
            />
          </div>
        )}
        <div className="e-field">
          <Slider
            min={0}
            max={5}
            step={0.1}
            value={+((effect.feather ?? DEFAULTS.feather) * 100).toFixed(1)}
            onChange={(v) => upd({ feather: v / 100 })}
            accentColor="var(--e-fx)"
            ariaLabel="Soft edge"
            label="Soft edge"
            formatValue={(v) => `${v.toFixed(1)}%`}
          />
        </div>
        <div className="e-field">
          <Slider
            min={0}
            max={50}
            step={1}
            value={Math.round((effect.roundness ?? DEFAULTS.roundness) * 100)}
            onChange={(v) => upd({ roundness: v / 100 })}
            accentColor="var(--e-fx)"
            ariaLabel="Corners"
            label="Corners"
            formatValue={(v) => `${Math.round(v)}%`}
          />
        </div>
      </Section>

      <Section title="Fades">
        <div className="e-field2">
          <label className="e-field">
            <span className="e-fl">Fade in</span>
            <NumberField
              step={0.05}
              min={0}
              max={span}
              value={secOf(effect.fade_in_ms)}
              onChange={(v) => upd({ fade_in_ms: Math.round(v * 1000) })}
            />
          </label>
          <label className="e-field">
            <span className="e-fl">Fade out</span>
            <NumberField
              step={0.05}
              min={0}
              max={span}
              value={secOf(effect.fade_out_ms)}
              onChange={(v) => upd({ fade_out_ms: Math.round(v * 1000) })}
            />
          </label>
        </div>
      </Section>
    </InspectorShell>
  );
}
