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
  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Audio" lede="Sync offset and volume for the System and Mic tracks." onClose={onClose} />

      <div className="e-field">
        <span className="e-fl">
          Mic Sync Offset <b>{offsetMs > 0 ? "+" : ""}{offsetMs} ms</b>
        </span>
        <div className="e-hintrow">
          <span>← Mic earlier</span>
          <span>Mic later →</span>
        </div>
        <Slider
          min={-300}
          max={300}
          step={10}
          value={offsetMs}
          onChange={onChangeOffset}
          ariaLabel="Mic Sync Offset"
        />
      </div>

      <div className="e-field" style={{ marginTop: 16 }}>
        <span className="e-fl">System Audio Volume <b>{Math.round(sysVol * 100)}%</b></span>
        <Slider
          min={0}
          max={150}
          step={5}
          value={Math.round(sysVol * 100)}
          onChange={(v) => onChangeSysVol(v / 100)}
          ariaLabel="System Audio Volume"
        />
      </div>

      <div className="e-field">
        <span className="e-fl">Microphone Volume <b>{Math.round(micVol * 100)}%</b></span>
        <Slider
          min={0}
          max={150}
          step={5}
          value={Math.round(micVol * 100)}
          onChange={(v) => onChangeMicVol(v / 100)}
          ariaLabel="Microphone Volume"
        />
      </div>
    </div>
  );
}
