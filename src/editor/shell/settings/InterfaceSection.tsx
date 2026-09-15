import { IconRotate2 } from "@tabler/icons-react";
import type { InterfaceSettings, ThemeMode } from "../../../hud/settings/settings";
import { Picker, Swatches, Switch, type SwatchItem } from "../../controls/Controls";
import { ACCENTS } from "../../panels/background/backgroundPresets";

const THEME_OPTS: { value: ThemeMode; label: string }[] = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "system", label: "System" },
];
const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
const ACCENT_ITEMS: SwatchItem<[number, number, number]>[] = ACCENTS.map((p) => ({
  key: rgb(p.rgb),
  css: rgb(p.rgb),
  value: p.rgb,
  ariaLabel: p.name,
}));

export const DEFAULT_INTERFACE_RESET: Pick<
  InterfaceSettings,
  "theme" | "accent" | "animated_brand" | "ai_choreography"
> = {
  theme: "light",
  accent: [239, 68, 68],
  animated_brand: true,
  ai_choreography: false,
};

export function InterfaceSection({
  value,
  onChange,
}: {
  value: InterfaceSettings;
  onChange: (v: InterfaceSettings) => void;
}) {
  const set = <K extends keyof InterfaceSettings>(k: K, v: InterfaceSettings[K]) =>
    onChange({ ...value, [k]: v });

  return (
    <div className="e-sec">
      <div className="e-secrow">
        <span className="e-sechead">Interface</span>
        <button
          type="button"
          className="e-hicon"
          title="Reset to default"
          aria-label="Reset interface settings"
          onClick={() => onChange({ ...value, ...DEFAULT_INTERFACE_RESET })}
        >
          <IconRotate2 size={14} />
        </button>
      </div>

      <div className="e-field">
        <span className="e-fl">Theme</span>
        <Picker
          value={value.theme}
          options={THEME_OPTS}
          onChange={(theme) => set("theme", theme)}
          ariaLabel="Theme"
        />
      </div>

      <div className="e-field">
        <span className="e-fl">Accent</span>
        <Swatches
          items={ACCENT_ITEMS}
          variant="accent"
          isSelected={(c) => rgb(value.accent) === rgb(c)}
          onSelect={(c) => set("accent", c)}
        />
      </div>

      <div className="e-field">
        <div className="e-switchrow">
          <span>Animated brand</span>
          <Switch on={value.animated_brand} onChange={(v) => set("animated_brand", v)} />
        </div>
        <span className="e-lede" style={{ margin: "6px 0 0" }}>
          Flowing wave + REC pulse on the brand mark.
        </span>
      </div>

      <div className="e-field" style={{ marginBottom: 0 }}>
        <div className="e-switchrow">
          <span>Replay applied edits with the pointer</span>
          <Switch on={value.ai_choreography} onChange={(v) => set("ai_choreography", v)} />
        </div>
        <span className="e-lede" style={{ margin: "6px 0 0" }}>
          Off by default. The edits are already applied; this only performs them.
        </span>
      </div>
    </div>
  );
}
