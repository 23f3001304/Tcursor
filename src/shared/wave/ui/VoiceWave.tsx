import { useEffect, useId, useRef } from "react";
import { LAYERS, ceilFor, initialVoiceState, layerPath, voiceFrame } from "../math/voiceWave";
import { useReducedMotion } from "./useReducedMotion";
import "../../wave.css";

const CAP_INSET = 2;
const CAP_R = 2;

const STILL_EPSILON_PX = 0.05;

export function VoiceWave({
  w = 104,
  h = 30,
  read,
  live,
}: {
  w?: number;
  h?: number;
  read: () => { mic: number; sys: number };
  live: boolean;
}) {
  const paths = useRef<(SVGPathElement | null)[]>([]);
  const reduced = useReducedMotion();
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
      st = voiceFrame(st, mic, sys, dt, ceilFor(h));
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
    <svg
      className={`w-wave w-voice${live ? "" : " stale"}`}
      width={w}
      height={h}
      viewBox={`0 0 ${w} ${h}`}
      aria-hidden="true"
      focusable="false"
    >
      <defs>
        <linearGradient id={gid} gradientUnits="userSpaceOnUse" x1={0} y1={0} x2={w} y2={0}>
          <stop className="w-grad-a" offset="0" />
          <stop className="w-grad-b" offset="1" />
        </linearGradient>
      </defs>
      <line className="w-axis" x1={0} y1={mid} x2={w} y2={mid} />
      {LAYERS.map((layer, i) => (
        <path
          key={i}
          ref={(el) => {
            paths.current[i] = el;
          }}
          className="w-layer"
          fill={`url(#${gid})`}
          fillOpacity={layer.alpha}
        />
      ))}
      <circle className="w-cap a" cx={CAP_INSET} cy={mid} r={CAP_R} />
      <circle className="w-cap b" cx={w - CAP_INSET} cy={mid} r={CAP_R} />
    </svg>
  );
}
