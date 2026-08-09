import { IconRotate2 } from "@tabler/icons-react";
import type { ModeAppearance } from "../../../hud/settings/settings";
import { Slider } from "../../controls/Controls";

// `ModeAppearance::default().screen_size` (settings/appearance.rs) - 1.0, i.e. the screen
// panel at full size. Reset here only ever touches `screen_size`; `pad`/`screen_radius` on
// the same `ModeAppearance` are BackgroundPanel's controls (frame padding/corner radius).
export const DEFAULT_SCREEN_SIZE = 1.0;

/** The one `appearance.screen` field with no other editor surface (T34 covers the other
 *  four layout modes' appearance; BackgroundPanel already owns `pad`/`screen_radius` on
 *  this same mode). `screen_size` scales the screen panel toward its own center - the
 *  0.6..1.0 range mirrors the Rust field comment exactly. */
export function ScreenSection({ value, onChange }: {
  value: ModeAppearance; onChange: (v: ModeAppearance) => void;
}) {
  const setSize = (screen_size: number) => onChange({ ...value, screen_size });

  return (
    <div className="e-sec">
      <div className="e-secrow">
        <span className="e-sechead">Screen</span>
        <button type="button" className="e-hicon" title="Reset to default" aria-label="Reset screen size"
          onClick={() => setSize(DEFAULT_SCREEN_SIZE)}>
          <IconRotate2 size={14} />
        </button>
      </div>
      <div className="e-field" style={{ marginBottom: 0 }}>
        <span className="e-fl">Screen size <b>{Math.round(value.screen_size * 100)}%</b></span>
        <Slider min={0.6} max={1} step={0.01} value={value.screen_size} onChange={setSize} ariaLabel="Screen size" />
      </div>
    </div>
  );
}
