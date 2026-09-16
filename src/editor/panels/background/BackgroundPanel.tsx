import { useEffect, useState } from "react";
import { PanelHeader } from "../PanelHeader";
import type { EditDoc } from "../../../shared/edit";
import type { BackgroundSettings } from "../../../hud/settings/settings";
import { backgroundThumbs, type BackgroundThumb } from "../../../shared/ipc";
import { ColorInput, Disclosure, Segmented, Slider, Swatches } from "../../controls/Controls";
import { COLOR_ITEMS, DEFAULT_BG, rgb } from "./backgroundPresets";
import { GradeSection, NO_GRADE } from "./GradeSection";
import { WallpaperTab } from "./WallpaperGrid";
import { GradientTab } from "./GradientTab";

type BgTab = "wallpapers" | "color" | "gradient";

const TABS: { value: BgTab; label: string }[] = [
  { value: "wallpapers", label: "Wallpapers" },
  { value: "color", label: "Color" },
  { value: "gradient", label: "Gradient" },
];

export function BackgroundPanel({
  folder,
  doc,
  onSaveSettings,
  onClose,
}: {
  folder: string;
  doc: EditDoc;
  onSaveSettings: (nextSettings: EditDoc["settings"]) => void;
  onClose: () => void;
}) {
  const app = doc.settings.appearance;
  const bg = doc.settings.background;
  const padPct = Math.round((app.screen?.pad ?? 0.03125) * 100);
  const radiusPx = Math.round((app.screen?.screen_radius ?? 0.016) * 1000);
  const [tab, setTab] = useState<BgTab>(
    bg.kind === "solid" ? "color" : bg.kind === "gradient" ? "gradient" : "wallpapers",
  );
  const [thumbs, setThumbs] = useState<BackgroundThumb[] | null>(null);
  useEffect(() => {
    let live = true;
    backgroundThumbs()
      .then((t) => {
        if (live) setThumbs(t);
      })
      .catch(() => {
        if (live) setThumbs([]);
      });
    return () => {
      live = false;
    };
  }, []);

  const setBg = (patch: Partial<BackgroundSettings>) =>
    onSaveSettings({ ...doc.settings, background: { ...bg, ...patch } });

  const handleReset = () => {
    setTab("wallpapers");
    onSaveSettings({
      ...doc.settings,
      background: DEFAULT_BG,
      grade: NO_GRADE,
      appearance: { ...app, screen: { ...app.screen, pad: 0.03125, screen_radius: 0.016 } },
    });
  };

  const setPad = (v: number) => {
    const frac = v / 100;
    onSaveSettings({ ...doc.settings, appearance: { ...app, screen: { ...app.screen, pad: frac } } });
  };

  const setRadius = (v: number) => {
    const frac = v / 1000;
    onSaveSettings({
      ...doc.settings,
      appearance: { ...app, screen: { ...app.screen, screen_radius: frac } },
    });
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader
        title="Background"
        lede="What sits behind the screen, its frame, and the look of the whole picture."
        onReset={handleReset}
        onClose={onClose}
      />

      <div className="e-grp">
        <span className="e-sechead">Background Type</span>
        <Segmented value={tab} options={TABS} onChange={setTab} ariaLabel="Background Type" />
      </div>

      {tab === "wallpapers" && <WallpaperTab folder={folder} bg={bg} thumbs={thumbs} setBg={setBg} />}

      {tab === "color" && (
        <div className="e-grp">
          <span className="e-sechead">Presets</span>
          <Swatches
            items={COLOR_ITEMS}
            variant="preset"
            isSelected={(c) => bg.kind === "solid" && rgb(bg.solid) === rgb(c)}
            onSelect={(c) => setBg({ kind: "solid", solid: c })}
          />
          <div className="e-colorrow">
            <ColorInput
              value={bg.solid}
              label="Custom"
              ariaLabel="Custom background color"
              onChange={(c) => setBg({ kind: "solid", solid: c })}
            />
          </div>
        </div>
      )}

      {tab === "gradient" && <GradientTab bg={bg} thumbs={thumbs} setBg={setBg} />}

      <Disclosure id="background">
        <div className="e-grp">
          <span className="e-sechead">Look</span>
          <div className="e-two">
            <div className="e-field">
              {bg.kind === "video" ? (
                <span className="e-hintline">Blur applies to still backgrounds.</span>
              ) : (
                <Slider
                  min={0}
                  max={100}
                  step={5}
                  value={Math.round(bg.blur * 100)}
                  onChange={(v) => setBg({ blur: v / 100 })}
                  ariaLabel="Background Blur"
                  label="Background Blur"
                  formatValue={(v) => `${Math.round(v)}%`}
                />
              )}
            </div>
            <div className="e-field">
              <Slider
                min={0}
                max={80}
                step={1}
                value={Math.round(bg.dim * 100)}
                onChange={(v) => setBg({ dim: v / 100 })}
                ariaLabel="Dim"
                label="Dim"
                formatValue={(v) => `${Math.round(v)}%`}
              />
            </div>
          </div>
        </div>

        <div className="e-grp">
          <span className="e-sechead">Frame</span>
          <div className="e-two">
            <div className="e-field">
              <Slider
                min={0}
                max={80}
                step={1}
                value={radiusPx}
                onChange={setRadius}
                ariaLabel="Corner Radius"
                label="Corner Radius"
                formatValue={(v) => `${Math.round(v)}px`}
              />
            </div>
            <div className="e-field">
              <Slider
                min={0}
                max={25}
                step={1}
                value={padPct}
                onChange={setPad}
                ariaLabel="Padding"
                label="Padding"
                formatValue={(v) => `${Math.round(v)}%`}
              />
            </div>
          </div>
        </div>
      </Disclosure>

      <GradeSection
        grade={doc.settings.grade}
        onChange={(grade) => onSaveSettings({ ...doc.settings, grade })}
      />
    </div>
  );
}
