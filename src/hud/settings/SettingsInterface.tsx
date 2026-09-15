import type { InterfaceSettings, ThemeMode } from "./settings";
import { Field, Switch } from "./SettingsControls";

const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;

const THEME_OPTS: { id: ThemeMode; label: string }[] = [
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
  { id: "system", label: "System" },
];

const ACCENTS: [number, number, number][] = [
  [239, 68, 68],
  [91, 91, 214],
  [47, 107, 255],
  [16, 185, 129],
  [245, 158, 11],
];

export function SettingsInterface({
  value,
  onChange,
}: {
  value: InterfaceSettings;
  onChange: (v: InterfaceSettings) => void;
}) {
  return (
    <section className="sec">
      <h3 className="sec-title">Interface</h3>

      <Field label="App theme">
        <div className="seg">
          {THEME_OPTS.map((t) => (
            <button
              key={t.id}
              type="button"
              className={`seg-btn ${value.theme === t.id ? "on" : ""}`}
              onClick={() => onChange({ ...value, theme: t.id })}
            >
              {t.label}
            </button>
          ))}
        </div>
      </Field>

      <Field label="Accent">
        <div className="swatches">
          {ACCENTS.map((c) => (
            <button
              key={rgb(c)}
              className={`swatch ${rgb(value.accent) === rgb(c) ? "on" : ""}`}
              style={{ background: rgb(c) }}
              onClick={() => onChange({ ...value, accent: c })}
              aria-label={rgb(c)}
            />
          ))}
        </div>
      </Field>

      <Field label="Animated brand" hint="Flowing wave + REC pulse">
        <Switch
          on={value.animated_brand}
          onChange={(animated_brand) => onChange({ ...value, animated_brand })}
        />
      </Field>

      <Field label="Interface effects" hint="Click ripples + magnetic controls">
        <Switch
          on={value.interface_effects}
          onChange={(interface_effects) => onChange({ ...value, interface_effects })}
        />
      </Field>

      <div className="sf">
        <div className="sf-row">
          <span className="sf-label">Language</span>
          <span className="sf-hint">English</span>
        </div>
      </div>
    </section>
  );
}
