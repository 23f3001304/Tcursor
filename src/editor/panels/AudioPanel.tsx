import { PanelHeader } from "./PanelHeader";
import { Slider } from "../controls/Controls";

export function AudioPanel({
  offsetMs,
  onChangeOffset,
  micVol,
  onChangeMicVol,
  sysVol,
  onChangeSysVol,
  onClose,
}: {
  offsetMs: number;
  onChangeOffset: (v: number) => void;
  micVol: number;
  onChangeMicVol: (v: number) => void;
  sysVol: number;
  onChangeSysVol: (v: number) => void;
  onClose: () => void;
}) {
  // Mirrors the Rust `Settings::default()` (settings/model.rs): audio_offset_ms 0, both volumes
  // 1.0 (unity gain, i.e. the sliders' 100%) - the only sane "no adjustment" state for these three.
  const handleReset = () => { onChangeOffset(0); onChangeMicVol(1); onChangeSysVol(1); };

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Audio" lede="Sync offset and volume for the System and Mic tracks."
        onReset={handleReset} onClose={onClose} />

      <div className="e-field">
        {/* Hint row moved BELOW the track (fix round 2) - Slider's own `label` always renders
            immediately above the track, so the row can no longer sit between the label and the
            track as it did with the old hand-rolled label; below the track reads fine too (an
            axis legend under the thing it describes). */}
        <Slider
          min={-300}
          max={300}
          step={10}
          value={offsetMs}
          onChange={onChangeOffset}
          ariaLabel="Mic Sync Offset"
          label="Mic Sync Offset"
          formatValue={(v) => `${v > 0 ? "+" : ""}${v} ms`}
        />
        <div className="e-hintrow">
          <span>← Mic earlier</span>
          <span>Mic later →</span>
        </div>
      </div>

      <div className="e-field" style={{ marginTop: 16 }}>
        <Slider
          min={0}
          max={150}
          step={5}
          value={Math.round(sysVol * 100)}
          onChange={(v) => onChangeSysVol(v / 100)}
          ariaLabel="System Audio Volume"
          label="System Audio Volume"
          formatValue={(v) => `${Math.round(v)}%`}
        />
      </div>

      <div className="e-field">
        <Slider
          min={0}
          max={150}
          step={5}
          value={Math.round(micVol * 100)}
          onChange={(v) => onChangeMicVol(v / 100)}
          ariaLabel="Microphone Volume"
          label="Microphone Volume"
          formatValue={(v) => `${Math.round(v)}%`}
        />
      </div>
    </div>
  );
}
