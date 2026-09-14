import type { ModeAppearance } from "../../hud/settings/settings";
import type { ModeKey } from "../../hud/preferences/appearanceFields";
import { LayoutPreview } from "../../hud/components/LayoutPreview";

/** The picked layout's schematic, at the panel's own width: the same screen/webcam boxes the HUD's
 *  Appearance settings draw, reused rather than re-derived. `LayoutPreview` is a pure component
 *  (mode + appearance in, two absolutely-positioned divs out), so the ONLY thing the editor has to
 *  supply is the surface those divs sit on - the HUD's `.lp-*` rules live in `hud/settings/
 *  settings.css`, which the editor does not load. `.e-laypv` in `panels.css` states them in editor
 *  tokens, and beats the HUD's own rules on specificity wherever both happen to be in a bundle.
 *
 *  Sharing the component is the point: the box arithmetic (padding, screen scale, bubble corner and
 *  margins, the 16:9 stage) is the one thing that must not drift between the two editors of the
 *  same five layouts. */
export function LayoutMiniPreview({ mode, ma }: { mode: ModeKey; ma: ModeAppearance }) {
  return (
    <div className="e-laypv" aria-hidden="true">
      <LayoutPreview mode={mode} ma={ma} />
    </div>
  );
}
