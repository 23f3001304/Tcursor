import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { PanelHeader } from "./PanelHeader";
import type { CursorBackStyle, CursorSettings, CursorStyle } from "../../hud/settings/settings";
import { Switch, Slider, Segmented, Disclosure } from "../controls/Controls";
import { Picker } from "../controls/Picker";
import { CursorPackField } from "./CursorPackField";

// design/premium-pass D6: the honesty hint below pops with a style choice - a cheap opacity/y-4
// tween, consistent with the existing dialog enters.
const HINT_MOTION = { initial: { opacity: 0, y: -4 }, animate: { opacity: 1, y: 0 }, exit: { opacity: 0, y: -4 }, transition: { duration: 0.14 } };

// System keeps the OS cursor already baked into the recording; Enhanced redraws a smooth synthetic
// pointer (from the recorded cursor path, captured for every style); Hidden shows none. Having all
// three here - not just Enhanced/Hidden - is what lets a clip recorded in System go back to its
// original cursor in the editor. The segment labels are the bare state names (three descriptions
// do not fit one 320px row); the description each option used to carry is now its tooltip.
const STYLE_OPTS: { value: CursorStyle; label: string; title: string }[] = [
  { value: "system", label: "System", title: "System - original cursor" },
  { value: "enhanced", label: "Enhanced", title: "Enhanced - redrawn" },
  { value: "hidden", label: "Hidden", title: "Hidden - no cursor" },
];

// Mirrors the Rust `CursorSettings::default()` (settings/model.rs) byte-for-byte, including
// `style: "system"` - a prior bug reset to `"enhanced"` here, silently diverging from the
// backend default every time a user pressed Reset.
// The glass shape behind the cursor. Its own setting, not the pack's: a plain pack can have one
// and a glass pack can go without. See Rust `settings::cursor::CursorBack`.
const BACK_OPTS: { value: CursorBackStyle; label: string; title: string }[] = [
  { value: "none", label: "None", title: "Nothing behind the cursor" },
  { value: "glass", label: "Glass", title: "A refracting disc - a pill over text, stretching along a selection" },
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

/** Panel flow (panel pass): what the cursor IS (style, then which pack), how big it is, how it
 *  moves, then what a click does - with every switch sitting directly above what it enables. */
export function CursorPanel({
  settings,
  onChange,
  onClose,
  // Optional so the panel still renders standalone (and in its own tests) as if the recording
  // did bake an OS cursor - i.e. no hint, today's wording.
  osCursorInVideo = true,
  hasCursorLayer = false,
}: {
  settings: CursorSettings;
  onChange: (v: CursorSettings) => void;
  onClose: () => void;
  osCursorInVideo?: boolean;
  /** This recording captured the real OS cursor as its own layer (`cursor_layer`), so "System"
   *  IS the original cursor - the "re-created" honesty hint below must not show. Only a pre-layer
   *  recording made in Enhanced/Hidden (no baked cursor, no layer) is genuinely re-created. */
  hasCursorLayer?: boolean;
}) {
  const still = useReducedMotion();
  const set = <K extends keyof CursorSettings>(k: K, v: CursorSettings[K]) => {
    onChange({ ...settings, [k]: v });
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Cursor" lede="Shape, size and motion of the pointer."
        onReset={() => onChange(DEFAULT_CURSOR_SETTINGS)} onClose={onClose} />

      {/* Style: System (keep the recorded OS cursor) / Enhanced (redraw) / Hidden. The pack, size,
          and motion groups below only apply to Enhanced, so they're hidden for the other two. */}
      <div className="e-grp">
        <span className="e-sechead">Style</span>
        <Segmented value={settings.style} options={STYLE_OPTS} onChange={(style) => set("style", style)} ariaLabel="Cursor style" />
        {/* Honesty hint: this recording has no OS cursor in its pixels, so System is re-created. */}
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

          {/* Directly under the pack, because it is the other half of "what the cursor IS": the
              pack decides the shape, this decides what sits behind it. */}
          <div className="e-grp">
            <span className="e-sechead">Back</span>
            <Picker value={settings.back} options={BACK_OPTS} onChange={(v) => set("back", v)} ariaLabel="Cursor back" />
          </div>

          <div className="e-grp">
            <span className="e-sechead">Size</span>
            <div className="e-field">
              <Slider min={0.4} max={3.0} step={0.1} value={settings.size} onChange={(v) => set("size", v)} ariaLabel="Cursor Size"
                label="Cursor Size" formatValue={(v) => `${v.toFixed(2)}x`} />
            </div>
          </div>

          {/* Motion and Click are set once and left: the pack and the size are what a user comes
              back to. Even as strips the two pack rows plus Style and Size fill 450px of a 620px
              slot, so these five controls go under the panel's one disclosure rather than push it
              over. Closed by default, remembered once opened. */}
          <Disclosure id="cursor">
            <div className="e-grp">
              <span className="e-sechead">Motion</span>
              <div className="e-field">
                <Slider min={0.0} max={1.0} step={0.05} value={settings.smoothness} onChange={(v) => set("smoothness", v)} ariaLabel="Cursor Smoothness"
                  label="Cursor Smoothness" formatValue={(v) => v.toFixed(2)} />
              </div>
              <div className="e-field">
                <Slider min={0.0} max={1.0} step={0.05} value={settings.path_idealize} onChange={(v) => set("path_idealize", v)} ariaLabel="Path Idealization"
                  label="Path Idealization" formatValue={(v) => v.toFixed(2)} />
              </div>
              <div className="e-field">
                <Slider min={0.0} max={1.0} step={0.05} value={settings.motion_blur} onChange={(v) => set("motion_blur", v)} ariaLabel="Motion Trail Blur"
                  label="Motion Trail Blur" formatValue={(v) => `${v.toFixed(2)}x`} />
              </div>
              {/* How far a thrown cursor tips into its own travel, and overshoots once coming back
                  upright when it stops (`export/cursor/tilt.rs`). 0 switches the filter off. */}
              <div className="e-field">
                <Slider min={0.0} max={1.0} step={0.05} value={settings.tilt} onChange={(v) => set("tilt", v)} ariaLabel="Motion Tilt"
                  label="Motion Tilt" formatValue={(v) => v.toFixed(2)} />
              </div>
            </div>

            {/* The bounce switch and the intensity it scales, together: the intensity slider used
                to sit alone at the bottom of the panel with its switch four groups above it. */}
            <div className="e-grp">
              <span className="e-sechead">Click</span>
              <div className="e-switchrow">
                <span>Click bounce animation</span>
                <Switch on={settings.click_bounce} onChange={(v) => set("click_bounce", v)} ariaLabel="Click bounce animation" />
              </div>
              <div className="e-field">
                <Slider min={0.1} max={1.0} step={0.05} value={settings.bounce_intensity} disabled={!settings.click_bounce}
                  onChange={(v) => set("bounce_intensity", v)} ariaLabel="Click Bounce Intensity"
                  label="Click Bounce Intensity" formatValue={(v) => `${v.toFixed(2)}x`} />
              </div>
            </div>
          </Disclosure>
        </>
      )}
    </div>
  );
}
