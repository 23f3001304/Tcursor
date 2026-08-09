import { IconRotate2 } from "@tabler/icons-react";
import type { InterfaceSettings, ThemeMode } from "../../../hud/settings/settings";
import { Picker, Switch } from "../../controls/Controls";

const THEME_OPTS: { value: ThemeMode; label: string }[] = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "system", label: "System" },
];

// `InterfaceSettings::default()` (settings/model.rs) minus `accent` - `theme`/`animated_brand`
// are this section's own fields; `accent` stays BackgroundPanel's (Accent Colors swatches),
// so Reset spreads only these two over the CURRENT value rather than replacing the whole object.
export const DEFAULT_INTERFACE_RESET: Pick<InterfaceSettings, "theme" | "animated_brand"> = {
  theme: "light",
  animated_brand: true,
};

/** `doc.settings.ui.theme`/`.animated_brand` - the doc-scoped copy of `InterfaceSettings`,
 *  NOT the HUD's own global app config (`getSettings`/`saveSettings`, applied via `applyTheme`
 *  on `document.documentElement` in `Hud.tsx`). Verified before building this: the Rust
 *  renderer already reads THIS per-project copy - `resolve_dark(settings.ui.theme)` in both
 *  `export/render/mod.rs` and `export/cursor/cursorpreview.rs` picks the dark/light synthetic
 *  cursor sprite baked into the export/preview - so it is a real per-project render setting,
 *  same family as `ui.accent` (already doc-scoped via `BackgroundPanel`) and `ui.animated_brand`
 *  (already read doc-scoped for `Editor`'s `brandState`). Writing it through `saveDocSettings`
 *  (the same call `BackgroundPanel`'s accent write uses) is therefore the consistent path - no
 *  Rust change needed, the field and its two consumers already exist. */
export function InterfaceSection({ value, onChange }: {
  value: InterfaceSettings; onChange: (v: InterfaceSettings) => void;
}) {
  const set = <K extends keyof InterfaceSettings>(k: K, v: InterfaceSettings[K]) => onChange({ ...value, [k]: v });

  return (
    <div className="e-sec">
      <div className="e-secrow">
        <span className="e-sechead">Interface</span>
        <button type="button" className="e-hicon" title="Reset to default" aria-label="Reset interface settings"
          onClick={() => onChange({ ...value, ...DEFAULT_INTERFACE_RESET })}>
          <IconRotate2 size={14} />
        </button>
      </div>

      <div className="e-field">
        <span className="e-fl">Theme</span>
        <Picker value={value.theme} options={THEME_OPTS} onChange={(theme) => set("theme", theme)} ariaLabel="Theme" />
      </div>

      <div className="e-field" style={{ marginBottom: 0 }}>
        <div className="e-switchrow">
          <span>Animated brand</span>
          <Switch on={value.animated_brand} onChange={(v) => set("animated_brand", v)} />
        </div>
        <span className="e-lede" style={{ margin: "6px 0 0" }}>Flowing wave + REC pulse on the brand mark.</span>
      </div>
    </div>
  );
}
