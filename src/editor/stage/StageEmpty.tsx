import { HorizonWave } from "../../shared/wave/ui/QuietWaves";

export function StageEmpty() {
  return (
    <div className="e-stage-empty">
      <HorizonWave w={220} h={34} />
      <span>Preparing preview</span>
    </div>
  );
}
