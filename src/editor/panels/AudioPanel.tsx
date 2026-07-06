import { PanelHeader } from "./PanelHeader";
import { Slider } from "../controls/Controls";

export function AudioPanel({
  offsetMs,
  onChangeOffset,
  onClose,
}: {
  offsetMs: number;
  onChangeOffset: (v: number) => void;
  onClose: () => void;
}) {
  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Audio" lede="Sync offset and volume for the System and Mic tracks." onClose={onClose} />

      <div className="e-field">
        <span className="e-fl">
          Mic Sync Offset <b>{offsetMs > 0 ? "+" : ""}{offsetMs} ms</b>
        </span>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, color: "var(--e-dim)", marginBottom: 4 }}>
          <span>← Mic earlier</span>
          <span>Mic later →</span>
        </div>
        <Slider
          min={-300}
          max={300}
          step={10}
          value={offsetMs}
          onChange={onChangeOffset}
          accentColor="var(--e-fg)"
        />
      </div>

      <div className="e-field" style={{ marginTop: 14 }}>
        <span className="e-fl">System Audio Volume <b>{100}%</b> (Mocked)</span>
        <Slider
          min={0}
          max={150}
          step={5}
          value={100}
          onChange={() => {}}
          accentColor="var(--e-fg)"
        />
      </div>

      <div className="e-field">
        <span className="e-fl">Microphone Volume <b>{100}%</b> (Mocked)</span>
        <Slider
          min={0}
          max={150}
          step={5}
          value={100}
          onChange={() => {}}
          accentColor="var(--e-fg)"
        />
      </div>

      <div className="e-field" style={{ marginTop: 8 }}>
        <span className="e-fl">Noise Suppression (Mocked)</span>
        <div className="e-seg">
          <button className="on">Off</button>
          <button>Low</button>
          <button>High</button>
        </div>
      </div>
    </div>
  );
}
