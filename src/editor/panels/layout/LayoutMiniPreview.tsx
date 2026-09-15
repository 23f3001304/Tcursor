import type { ModeAppearance } from "../../../hud/settings/settings";
import type { ModeKey } from "../../../hud/preferences/appearanceFields";
import { LayoutPreview } from "../../../hud/components/LayoutPreview";

export function LayoutMiniPreview({ mode, ma }: { mode: ModeKey; ma: ModeAppearance }) {
  return (
    <div className="e-laypv" aria-hidden="true">
      <LayoutPreview mode={mode} ma={ma} />
    </div>
  );
}
