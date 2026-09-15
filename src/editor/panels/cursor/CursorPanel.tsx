import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { PanelHeader } from "../PanelHeader";
import type { CursorBackStyle, CursorSettings, CursorStyle } from "../../../hud/settings/settings";
import { Switch, Slider, Segmented, Disclosure } from "../../controls/Controls";
import { Picker } from "../../controls/fields/Picker";
import { CursorPackField } from "./CursorPackField";

const HINT_MOTION = {
  initial: { opacity: 0, y: -4 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -4 },
  transition: { duration: 0.14 },
};

const STYLE_OPTS: { value: CursorStyle; label: string; title: string }[] = [
  { value: "system", label: "System", title: "System - original cursor" },
  { value: "enhanced", label: "Enhanced", title: "Enhanced - redrawn" },
  { value: "hidden", label: "Hidden", title: "Hidden - no cursor" },
];

const BACK_OPTS: { value: CursorBackStyle; label: string; title: string }[] = [
  { value: "none", label: "None", title: "Nothing behind the cursor" },
  {
    value: "glass",
    label: "Glass",
    title: "A refracting disc - a pill over text, stretching along a selection",
  },
];

export const DEFAULT_CURSOR_SETTINGS: CursorSettings = {
  style: "system",
  size: 1.0,
  smoothness: 0.6,
  path_idealize: 0.0,
  motion_blur: 0.35,
  tilt: 0.35,
  click_bounce: true,
  bounce_intensity: 0.5,
  pack: "default",
  back: "none",
};

export function CursorPanel({
  settings,
  onChange,
  onClose,
  osCursorInVideo = true,
  hasCursorLayer = false,
}: {
  settings: CursorSettings;
  onChange: (v: CursorSettings) => void;
  onClose: () => void;
  osCursorInVideo?: boolean;
  hasCursorLayer?: boolean;
}) {
  const still = useReducedMotion();
  const set = <K extends keyof CursorSettings>(k: K, v: CursorSettings[K]) => {
    onChange({ ...settings, [k]: v });
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader
        title="Cursor"
        lede="Shape, size and motion of the pointer."
        onReset={() => onChange(DEFAULT_CURSOR_SETTINGS)}
        onClose={onClose}
      />

      <div className="e-grp">
        <span className="e-sechead">Style</span>
        <Segmented
          value={settings.style}
          options={STYLE_OPTS}
          onChange={(style) => set("style", style)}
          ariaLabel="Cursor style"
        />
        <AnimatePresence>
          {settings.style === "system" && !osCursorInVideo && !hasCursorLayer && (
            <motion.p className="e-hintline" {...(still ? {} : HINT_MOTION)}>
              Re-created from the recorded path. This clip was recorded without the system cursor.
            </motion.p>
          )}
        </AnimatePresence>
      </div>

      {settings.style === "enhanced" && (
        <>
          <CursorPackField pack={settings.pack} onPick={(id) => set("pack", id)} />

          <div className="e-grp">
            <span className="e-sechead">Back</span>
            <Picker
              value={settings.back}
              options={BACK_OPTS}
              onChange={(v) => set("back", v)}
              ariaLabel="Cursor back"
            />
          </div>

          <div className="e-grp">
            <span className="e-sechead">Size</span>
            <div className="e-field">
              <Slider
                min={0.4}
                max={3.0}
                step={0.1}
                value={settings.size}
                onChange={(v) => set("size", v)}
                ariaLabel="Cursor Size"
                label="Cursor Size"
                formatValue={(v) => `${v.toFixed(2)}x`}
              />
            </div>
          </div>

          <Disclosure id="cursor">
            <div className="e-grp">
              <span className="e-sechead">Motion</span>
              <div className="e-field">
                <Slider
                  min={0.0}
                  max={1.0}
                  step={0.05}
                  value={settings.smoothness}
                  onChange={(v) => set("smoothness", v)}
                  ariaLabel="Cursor Smoothness"
                  label="Cursor Smoothness"
                  formatValue={(v) => v.toFixed(2)}
                />
              </div>
              <div className="e-field">
                <Slider
                  min={0.0}
                  max={1.0}
                  step={0.05}
                  value={settings.path_idealize}
                  onChange={(v) => set("path_idealize", v)}
                  ariaLabel="Path Idealization"
                  label="Path Idealization"
                  formatValue={(v) => v.toFixed(2)}
                />
              </div>
              <div className="e-field">
                <Slider
                  min={0.0}
                  max={1.0}
                  step={0.05}
                  value={settings.motion_blur}
                  onChange={(v) => set("motion_blur", v)}
                  ariaLabel="Motion Trail Blur"
                  label="Motion Trail Blur"
                  formatValue={(v) => `${v.toFixed(2)}x`}
                />
              </div>
              <div className="e-field">
                <Slider
                  min={0.0}
                  max={1.0}
                  step={0.05}
                  value={settings.tilt}
                  onChange={(v) => set("tilt", v)}
                  ariaLabel="Motion Tilt"
                  label="Motion Tilt"
                  formatValue={(v) => v.toFixed(2)}
                />
              </div>
            </div>

            <div className="e-grp">
              <span className="e-sechead">Click</span>
              <div className="e-switchrow">
                <span>Click bounce animation</span>
                <Switch
                  on={settings.click_bounce}
                  onChange={(v) => set("click_bounce", v)}
                  ariaLabel="Click bounce animation"
                />
              </div>
              <div className="e-field">
                <Slider
                  min={0.1}
                  max={1.0}
                  step={0.05}
                  value={settings.bounce_intensity}
                  disabled={!settings.click_bounce}
                  onChange={(v) => set("bounce_intensity", v)}
                  ariaLabel="Click Bounce Intensity"
                  label="Click Bounce Intensity"
                  formatValue={(v) => `${v.toFixed(2)}x`}
                />
              </div>
            </div>
          </Disclosure>
        </>
      )}
    </div>
  );
}
