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
  const handleReset = () => {
    onChangeOffset(0);
    onChangeMicVol(1);
    onChangeSysVol(1);
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader
        title="Audio"
        lede="Levels for the two tracks, and their sync."
        onReset={handleReset}
        onClose={onClose}
      />

      <div className="e-grp">
        <span className="e-sechead">Volume</span>
        <div className="e-field">
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

      <div className="e-grp">
        <span className="e-sechead">Sync</span>
        <div className="e-field">
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
            <span>&larr; Mic earlier</span>
            <span>Mic later &rarr;</span>
          </div>
        </div>
      </div>
    </div>
  );
}
