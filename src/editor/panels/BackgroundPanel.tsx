import { useEffect, useState } from "react";
import { PanelHeader } from "./PanelHeader";
import type { EditDoc } from "../../lib/edit";
import type { BackgroundSettings } from "../../hud/settings/settings";
import { backgroundThumbs, type BackgroundThumb } from "../../lib/ipc";
import { ColorInput, Disclosure, Segmented, Slider, Swatches, type SwatchItem } from "../controls/Controls";
import { COLOR_PRESETS, ACCENTS, DEFAULT_BG } from "./backgroundPresets";
import { WallpaperTab } from "./WallpaperGrid";
import { GradientTab } from "./GradientTab";

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
  // `null` = still loading (the tabs show skeleton tiles); `[]` = resolved with nothing (no ffmpeg,
  // an older backend), which the tabs render as the plain-swatch fallback. Since 2026-09-14 the
  // backend keeps the rendered set on disk and warms it at startup, so this normally resolves at
  // once; on the very first launch it still takes seconds, and the owner saw the panel sit on a
  // lone Classic tile under "Wallpapers 1" for all of them.
  const [thumbs, setThumbs] = useState<BackgroundThumb[] | null>(null);
  useEffect(() => {
    let live = true;
    backgroundThumbs().then((t) => { if (live) setThumbs(t); }).catch(() => { if (live) setThumbs([]); });
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

      {/* One collapsible SECTION per wallpaper group (Ribbons / Folds / Gradients / Metal /
          Scenic), in the order the backend lists them, then the user's own file last. The stack
          lives in `WallpaperTab` (`WallpaperGrid.tsx`) so this file stays a flat read of the
          panel's flow. */}
      {tab === "wallpapers" && <WallpaperTab folder={folder} bg={bg} thumbs={thumbs} setBg={setBg} />}

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
          lives under the panel's one disclosure. That is not a preference: Look, Frame and Accent
          together are 215px, and the choosing stack above (six section headers plus the one open
          section) already fills most of a 620px slot. Choosing stays in sight; tuning is one click
          away and remembered, so a user who tunes often never sees it closed again.
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
