// BackgroundPanel's Gradient tab: the curated presets on top, the user's own gradient below.
// Both write the SAME four settings fields (`gradient_from`/`gradient_mid`/`gradient_to`/
// `gradient_angle_deg`), so picking a preset and then nudging a stop is one continuous edit
// rather than two modes - the preset grid simply stops showing a selection once the colours no
// longer match any of them.
import type { BackgroundSettings } from "../../hud/settings/settings";
import type { BackgroundThumb } from "../../lib/ipc";
import { CategorySection, ColorInput, defaultOpenIndex, Slider } from "../controls/Controls";
import { thumbGroups, TileGrid, TileGridSkeleton } from "./WallpaperGrid";

type Rgb = [number, number, number];
const same = (a: Rgb | null | undefined, b: Rgb | null | undefined) =>
  a === b || (!!a && !!b && a[0] === b[0] && a[1] === b[1] && a[2] === b[2]);

/** Is `bg` exactly this preset? Both the middle stop's absence and the angle count, so a preset
 *  the user has since edited correctly reads as "no preset selected". */
export function isPreset(t: BackgroundThumb, bg: BackgroundSettings): boolean {
  const g = t.gradient;
  if (!g) return false;
  return same(g.from, bg.gradient_from) && same(g.to, bg.gradient_to)
    && same(g.mid ?? null, bg.gradient_mid ?? null) && g.angle_deg === bg.gradient_angle_deg;
}

export function GradientTab({
  bg,
  thumbs,
  setBg,
}: {
  bg: BackgroundSettings;
  /** `null` while the thumbnails are still loading. */
  thumbs: BackgroundThumb[] | null;
  setBg: (patch: Partial<BackgroundSettings>) => void;
}) {
  if (thumbs === null) {
    return (
      <div className="e-grp e-secstack">
        <CategorySection id="bg.gradient.loading" label="Presets" count={0} defaultOpen><TileGridSkeleton /></CategorySection>
      </div>
    );
  }
  const presets = thumbs.filter((t) => t.kind === "gradient");
  const selectedId = bg.kind === "gradient" ? (presets.find((t) => isPreset(t, bg))?.id ?? null) : null;
  const mid = bg.gradient_mid ?? null;
  const groups = thumbGroups(thumbs, "gradient");
  const open = defaultOpenIndex(groups.map((g) => g.tiles.some((t) => t.id === selectedId)));

  const apply = (id: string) => {
    const g = presets.find((t) => t.id === id)?.gradient;
    if (!g) return;
    setBg({ kind: "gradient", gradient_from: g.from, gradient_mid: g.mid ?? null,
      gradient_to: g.to, gradient_angle_deg: g.angle_deg });
  };
  // Editing a stop also asserts `kind`, so tweaking a colour while another kind is active
  // switches to the gradient instead of silently changing a background nobody can see.
  const edit = (patch: Partial<BackgroundSettings>) => setBg({ kind: "gradient", ...patch });

  return (
    <>
      {/* The presets as collapsible sections, the same ones the Wallpapers tab is made of (the
          backend puts all twelve in one group, "Presets"). The section header IS the heading, so
          there is no `.e-sechead` above it saying the same word. */}
      <div className="e-grp e-secstack">
        {groups.map((g, i) => (
          <CategorySection key={g.name} id={`bg.grad.${g.name}`} label={g.name} count={g.tiles.length}
            selectedName={g.tiles.find((t) => t.id === selectedId)?.name ?? null} defaultOpen={i === open}>
            <TileGrid tiles={g.tiles} selectedId={selectedId} onSelect={apply} />
          </CategorySection>
        ))}
      </div>

      <div className="e-grp">
        <span className="e-sechead">Custom</span>
        <div className="e-colorrow">
          <ColorInput value={bg.gradient_from} label="From" ariaLabel="Gradient start color"
            onChange={(c) => edit({ gradient_from: c })} />
          {mid && (
            <ColorInput value={mid} label="Middle" ariaLabel="Gradient middle color"
              onChange={(c) => edit({ gradient_mid: c })} />
          )}
          <ColorInput value={bg.gradient_to} label="To" ariaLabel="Gradient end color"
            onChange={(c) => edit({ gradient_to: c })} />
        </div>
        <button type="button" className="e-ghostbtn"
          onClick={() => edit({ gradient_mid: mid ? null : midpoint(bg) })}>
          {mid ? "Remove middle stop" : "Add middle stop"}
        </button>
        <div className="e-field">
          <Slider min={0} max={360} step={5} value={Math.round(bg.gradient_angle_deg)}
            onChange={(v) => edit({ gradient_angle_deg: v })} ariaLabel="Gradient Angle"
            label="Gradient Angle" formatValue={(v) => `${Math.round(v)} deg`} />
        </div>
      </div>
    </>
  );
}

/** The colour a freshly added middle stop starts at: the average of the two ends, so switching it
 *  on changes nothing visible until the user actually moves it. */
function midpoint(bg: BackgroundSettings): Rgb {
  const f = bg.gradient_from, t = bg.gradient_to;
  return [Math.round((f[0] + t[0]) / 2), Math.round((f[1] + t[1]) / 2), Math.round((f[2] + t[2]) / 2)];
}
