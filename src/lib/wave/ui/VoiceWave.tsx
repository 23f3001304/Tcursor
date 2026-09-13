import { useEffect, useId, useRef } from "react";
import { LAYERS, initialVoiceState, layerPath, voiceFrame } from "../math/voiceWave";
import { useReducedMotion } from "./useReducedMotion";
import "../../wave.css";

/** The end dots: their x inset from each edge, and their radius. They sit ON the centre line,
 *  which the taper guarantees the wave also touches there, so the three read as one object. */
const CAP_INSET = 2;
const CAP_R = 2;
/** Under reduced motion the phase is frozen, so a frame is only worth redrawing when the level
 *  itself moved. Half a tenth of a px is under a display pixel at any DPI we ship. */
const STILL_EPSILON_PX = 0.05;

/** The recorder's live level meter: a centre line with a dot at each end, and four overlapping
 *  translucent lenses across the middle that swell with the input.
 *
 *  Each lens is a closed mirrored sine (`lensPath`) filled from one blue-to-violet ramp shared by
 *  all four (`gradientUnits="userSpaceOnUse"`, so the ramp belongs to the meter and not to each
 *  layer's own bounding box - otherwise a short layer would show the whole blue-to-violet span
 *  inside its own width and the colours would not line up). The layers differ only in phase,
 *  wavelength, amplitude and drift direction, so they sweep through each other; where two overlap
 *  the colour deepens because two translucent fills composited, not because anything painted a
 *  third colour there.
 *
 *  Everything that decides what a frame looks like is in `voiceWave` - this component is a rAF
 *  loop and four `setAttribute` calls, and deliberately never calls `setState`: at 60fps a React
 *  render per frame would re-render the whole recording bar. The loop lives and dies with the
 *  mount, and `RecMeter` only mounts while a take is running, so nothing is drawn when the HUD is
 *  not recording.
 *
 *  `read` is called once per frame for the newest RMS pair, so the caller can keep levels in a ref
 *  and never re-render on the 20Hz feed. `live` is the honesty gate: false (a source is enabled
 *  but no level has arrived) drops the layers to the line colour rather than letting a flat
 *  blue-to-violet wave imply capture that is not happening. */
export function VoiceWave({ w = 104, h = 30, read, live }: {
  w?: number; h?: number; read: () => { mic: number; sys: number }; live: boolean;
}) {
  const paths = useRef<(SVGPathElement | null)[]>([]);
  const reduced = useReducedMotion();
  // SVG fragment ids have to survive `url(#...)`, and React's generated ids carry punctuation.
  const gid = `vw${useId().replace(/[^a-zA-Z0-9]/g, "")}`;

  useEffect(() => {
    let raf = 0;
    let prev = performance.now();
    let drawnAmp = -1;
    let st = initialVoiceState(reduced);
    const tick = (now: number) => {
      raf = requestAnimationFrame(tick);
      const dt = (now - prev) / 1000;
      prev = now;
      const { mic, sys } = read();
      st = voiceFrame(st, mic, sys, dt);
      if (reduced && Math.abs(st.amp - drawnAmp) < STILL_EPSILON_PX) return;
      drawnAmp = st.amp;
      for (let i = 0; i < LAYERS.length; i++) {
        paths.current[i]?.setAttribute("d", layerPath(st, LAYERS[i], w, h));
      }
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [reduced, read, w, h]);

  const mid = h / 2;
  return (
    <svg className={`w-wave w-voice${live ? "" : " stale"}`} width={w} height={h}
      viewBox={`0 0 ${w} ${h}`} aria-hidden="true" focusable="false">
      <defs>
        <linearGradient id={gid} gradientUnits="userSpaceOnUse" x1={0} y1={0} x2={w} y2={0}>
          <stop className="w-grad-a" offset="0" />
          <stop className="w-grad-b" offset="1" />
        </linearGradient>
      </defs>
      <line className="w-axis" x1={0} y1={mid} x2={w} y2={mid} />
      {LAYERS.map((layer, i) => (
        <path key={i} ref={(el) => { paths.current[i] = el; }} className="w-layer"
          fill={`url(#${gid})`} fillOpacity={layer.alpha} />
      ))}
      <circle className="w-cap a" cx={CAP_INSET} cy={mid} r={CAP_R} />
      <circle className="w-cap b" cx={w - CAP_INSET} cy={mid} r={CAP_R} />
    </svg>
  );
}
