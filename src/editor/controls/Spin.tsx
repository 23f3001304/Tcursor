import { IdleWave } from "../../lib/wave/ui/QuietWaves";

/** The app's generic "working" indicator, used for preview/export/AI loading. Same API it always
 *  had (`size`), so every existing `<Spin>` inherits the wave motif without a call-site change -
 *  it is now the brand's own idle wave with the breathing dot rather than a rotating loader ring
 *  (benchmark (c) item 2: nobody else in the category connects their mark to their busy states).
 *  `IdleWave` owns the drawing and the reduced-motion behaviour. */
export function Spin({ size = 18 }: { size?: number }) {
  return <IdleWave size={size} />;
}
