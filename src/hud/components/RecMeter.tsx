import { Mic, MicOff } from "./icons";

/** The recording bar's mic indicator. Split out of `Hud.tsx` to keep that file under the line
 *  cap; owns no state - purely a rendering of the truth its props already carry.
 *
 *  State honesty (task-6 (c)/(i), user-reported): the live waveform renders ONLY when the mic is
 *  actually capturing (`micOn` AND the underlying stream is open, i.e. `active`) - when muted, an
 *  explicit "Muted" chip replaces it instead of a fake/frozen wave. No path here can render bars
 *  for a stream that was never opened; `Hud` only calls `useMicWaveform` with `on` when `micOn` is
 *  also true, so muting closes the mic for real, not just this component's display of it.
 *
 *  Bar geometry (gate-feedback item 2, user-reported 2026-09-02: "recording looks bad" - the old
 *  flex-grow bars, spread thin across a wide `.wave`, read as sparse dots rather than a meter):
 *  each bar is a FIXED `2 + level*18` px tall (hud.css floors it at `min-height: 2px` too, so
 *  total silence still shows a low bar, never a bare dot) - the fixed 3px-wide/2px-gap bar sizing
 *  and the meter's own fixed ~64px width live in `hud.css` (`.wave`/`.wave span`), not here. */
export function RecMeter({ micOn, active, levels }: { micOn: boolean; active: boolean; levels: number[] }) {
  return (
    <div className={`recmeter ${micOn ? "" : "muted"}`}>
      <span className="ico">{micOn ? <Mic /> : <MicOff />}</span>
      {micOn ? (
        <div className={`wave ${active ? "" : "idle"}`}>
          {levels.map((l, i) => <span key={i} style={{ height: `${2 + l * 18}px` }} />)}
        </div>
      ) : (
        <span className="muted-label">Muted</span>
      )}
    </div>
  );
}
