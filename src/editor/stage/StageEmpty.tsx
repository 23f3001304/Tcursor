import { HorizonWave } from "../../lib/wave/ui/QuietWaves";

/** The stage before the preview proxy exists: the wave motif's horizon line with the brand dot
 *  landing on it, over the one line of copy that says what is happening.
 *
 *  Its own component rather than an inline branch in `Stage.tsx` (which sits at its line cap), and
 *  the only empty state the editor has - benchmark (c) item 6: every competitor's empty state is
 *  inert, so a motion-led one is unclaimed ground. The wave replaced a generic loader ring here. */
export function StageEmpty() {
  return (
    <div className="e-stage-empty">
      <HorizonWave w={220} h={34} />
      <span>Preparing preview</span>
    </div>
  );
}
