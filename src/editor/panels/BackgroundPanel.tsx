import { useEffect, useState } from "react";
import { PanelHeader } from "./PanelHeader";
import type { EditDoc } from "../../lib/edit";
import type { BackgroundSettings } from "../../hud/settings/settings";
import { backgroundThumbs, type BackgroundThumb } from "../../lib/ipc";
import { ColorInput, Disclosure, Segmented, Slider, Swatches, type SwatchItem } from "../controls/Controls";
import { COLOR_PRESETS, ACCENTS, DEFAULT_BG } from "./backgroundPresets";
import { WallpaperRow, wallpaperGroups } from "./WallpaperGrid";
import { GradientTab } from "./GradientTab";
import { BackgroundAssetCard } from "./BackgroundAssetCard";

type BgTab = "wallpapers" | "color" | "gradient";
// Three exclusive kinds, so a segmented row rather than a dropdown (panel-design benchmark (b)2).
const TABS: { value: BgTab; label: string }[] = [
  { value: "wallpapers", label: "Wallpapers" }, { value: "color", label: "Color" }, { value: "gradient", label: "Gradient" },
];
const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;

// `ariaLabel` uses each preset's own name (COLOR_PRESETS/ACCENTS carry one).
const COLOR_ITEMS: SwatchItem<[number, number, number]>[] = COLOR_PRESETS.map((p) => ({ key: rgb(p.rgb), css: rgb(p.rgb), value: p.rgb, ariaLabel: p.name }));
const ACCENT_ITEMS: SwatchItem<[number, number, number]>[] = ACCENTS.map((p) => ({ key: rgb(p.rgb), css: rgb(p.rgb), value: p.rgb, ariaLabel: p.name }));

export function BackgroundPanel({
  folder,
  doc,
  onSaveSettings,
  onClose,
}: {
  /** The recording's folder: an imported background lives inside it (`background/<file>`). */
  folder: string;
  doc: EditDoc;
  onSaveSettings: (nextSettings: EditDoc["settings"]) => void;
  onClose: () => void;
}) {
  const app = doc.settings.appearance;
  const bg = doc.settings.background;
  const padPct = Math.round((app.screen?.pad ?? 0.03125) * 100);
  const radiusPx = Math.round((app.screen?.screen_radius ?? 0.016) * 1000);
  // Which preset grid is showing. Derived from the saved kind so reopening the panel lands on
  // the right tab, but merely BROWSING a tab never saves - only picking a tile/colour does.
  // `image`/`video` land on Wallpapers too: that is where the asset card lives, so reopening
  // the panel on an imported background shows the thing that is actually selected.
  const [tab, setTab] = useState<BgTab>(bg.kind === "solid" ? "color" : bg.kind === "gradient" ? "gradient" : "wallpapers");
  // Rendered once per process in Rust, so this refetch on every mount is a cheap clone. An error
  // (no ffmpeg, command missing) leaves the list empty: the grids still render their tiles, just
  // without artwork, so nothing becomes unselectable.
  const [thumbs, setThumbs] = useState<BackgroundThumb[]>([]);
  useEffect(() => {
    let live = true;
    backgroundThumbs().then((t) => { if (live) setThumbs(t); }).catch(() => {});
    return () => { live = false; };
  }, []);

  const setBg = (patch: Partial<BackgroundSettings>) =>
    onSaveSettings({ ...doc.settings, background: { ...bg, ...patch } });

  const handleReset = () => {
    setTab("wallpapers");
    onSaveSettings({
      ...doc.settings,
      background: DEFAULT_BG,
      appearance: { ...app, screen: { ...app.screen, pad: 0.03125, screen_radius: 0.016 } },
      ui: { ...doc.settings.ui, accent: [239, 68, 68] },
    });
  };

  const setPad = (v: number) => {
    const frac = v / 100;
    onSaveSettings({ ...doc.settings, appearance: { ...app, screen: { ...app.screen, pad: frac } } });
  };

  const setRadius = (v: number) => {
    const frac = v / 1000;
    onSaveSettings({ ...doc.settings, appearance: { ...app, screen: { ...app.screen, screen_radius: frac } } });
  };

  const setAccent = (c: [number, number, number]) => {
    onSaveSettings({ ...doc.settings, ui: { ...doc.settings.ui, accent: c } });
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Background" lede="What sits behind the screen, and its frame."
        onReset={handleReset} onClose={onClose} />

      {/* What the background IS, first: the kind, then the choices of that kind. */}
      <div className="e-grp">
        <span className="e-sechead">Background Type</span>
        <Segmented value={tab} options={TABS} onChange={setTab} ariaLabel="Background Type" />
      </div>

      {/* One ROW per wallpaper group (Ribbons / Folds / Gradients / Metal / Scenic), in the order
          the backend lists them; Classic (the legacy bundled mesh, id "") leads the first one,
          since it is what every pre-library project already renders. The five groups plus Custom
          are one block at 8px, not six sections at 16 - they are one library, read top to bottom.
          The user's own file is the last row rather than a section of its own: it is one more way
          to choose a background. Picking a wallpaper leaves `asset` alone, so coming back to that
          row restores it without a re-import. */}
      {tab === "wallpapers" && (
        <div className="e-grp e-rowstack">
          {wallpaperGroups(thumbs).map((g) => (
            <WallpaperRow key={g.name} label={g.name} tiles={g.tiles}
              selectedId={bg.kind === "mesh" ? bg.mesh : null}
              onSelect={(id) => setBg({ kind: "mesh", mesh: id })} />
          ))}
          <BackgroundAssetCard folder={folder} asset={bg.asset} kind={bg.kind}
            onPick={(k) => setBg({ kind: k })}
            onImported={(i) => setBg({ kind: i.kind, asset: i.rel_path })}
            onRemoved={() => setBg({ kind: "mesh", asset: null })} />
        </div>
      )}

      {tab === "color" && (
        <div className="e-grp">
          <span className="e-sechead">Presets</span>
          <Swatches items={COLOR_ITEMS} variant="preset"
            isSelected={(c) => bg.kind === "solid" && rgb(bg.solid) === rgb(c)}
            onSelect={(c) => setBg({ kind: "solid", solid: c })} />
          <div className="e-colorrow">
            <ColorInput value={bg.solid} label="Custom" ariaLabel="Custom background color"
              onChange={(c) => setBg({ kind: "solid", solid: c })} />
          </div>
        </div>
      )}

      {tab === "gradient" && <GradientTab bg={bg} thumbs={thumbs} setBg={setBg} />}

      {/* Everything below is TUNING - it adjusts a background you have already chosen - so it
          lives under the panel's one disclosure. That is not a preference: with five wallpaper
          rows plus Custom the panel's visible stack is 576px of a 620px slot, and Look, Frame and
          Accent together are another 215. Choosing stays in sight; tuning is one click away and
          remembered, so a user who tunes often never sees it closed again.
          Blur is a one-off pass over the STATIC background buffer, so on a video background it
          would only ever reach the first frame - a control that visibly does nothing reads as
          broken, so it is replaced by the reason it is absent. Dim is a black overlay over
          whatever has pixels, so it sits with Blur rather than inside any one tab. */}
      <Disclosure id="background">
        <div className="e-grp">
          <span className="e-sechead">Look</span>
          <div className="e-two">
            <div className="e-field">
              {bg.kind === "video" ? (
                <span className="e-hintline">Blur applies to still backgrounds.</span>
              ) : (
                <Slider min={0} max={100} step={5} value={Math.round(bg.blur * 100)} onChange={(v) => setBg({ blur: v / 100 })} ariaLabel="Background Blur"
                  label="Background Blur" formatValue={(v) => `${Math.round(v)}%`} />
              )}
            </div>
            <div className="e-field">
              <Slider min={0} max={80} step={1} value={Math.round(bg.dim * 100)} onChange={(v) => setBg({ dim: v / 100 })} ariaLabel="Dim"
                label="Dim" formatValue={(v) => `${Math.round(v)}%`} />
            </div>
          </div>
        </div>

        {/* Then the frame the screen sits in. */}
        <div className="e-grp">
          <span className="e-sechead">Frame</span>
          <div className="e-two">
            <div className="e-field">
              <Slider min={0} max={80} step={1} value={radiusPx} onChange={setRadius} ariaLabel="Corner Radius"
                label="Corner Radius" formatValue={(v) => `${Math.round(v)}px`} />
            </div>
            <div className="e-field">
              <Slider min={0} max={25} step={1} value={padPct} onChange={setPad} ariaLabel="Padding"
                label="Padding" formatValue={(v) => `${Math.round(v)}%`} />
            </div>
          </div>
        </div>

        {/* Last: the editor's own accent, which is not part of the background at all. */}
        <div className="e-grp">
          <span className="e-sechead">Accent Colors</span>
          <Swatches items={ACCENT_ITEMS} variant="accent"
            isSelected={(c) => rgb(doc.settings.ui.accent) === rgb(c)} onSelect={setAccent} />
        </div>
      </Disclosure>
    </div>
  );
}
