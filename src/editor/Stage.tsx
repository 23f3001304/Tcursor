import { useEffect, useRef, useState, type CSSProperties } from "react";
import { IconAspectRatio, IconClick, IconTypography, IconVideo } from "@tabler/icons-react";
import type { CamSample, ClickSample, CursorSpriteDto, CursorKindSample, PreviewLayout, HoldSpan } from "../lib/ipc";
import type { CursorSettings, ClickFxSettings } from "../hud/settings";
import type { EffectRegion } from "../lib/edit";
import { camAt } from "./camera";
import { drawPreview } from "./previewCanvas";
import { Spin } from "./Spin";

const MEDIA_ERR = ["", "aborted", "network", "decode", "src not supported (asset protocol blocked?)"];
const HIDDEN: CSSProperties = { position: "absolute", width: 1, height: 1, opacity: 0, pointerEvents: "none" };

/** Smooth, full-composite preview: the screen (a low-res proxy) and webcam play in hidden
 *  native <video>s, and each animation frame is composited onto a 2D canvas (background +
 *  rounded zoomed screen + webcam PiP) via drawPreview. The zoom comes from the exact
 *  camera_track curve. Native decode + Canvas2D drawImage = 60fps. */
export function Stage({ src, webcamSrc, track, layout, clicks, bgUrl, cursorSprites, cursorKinds, cursor, effects, spotlightHolds, clickfx, audioSrc, muted, timeMs, playing, onTime, onDuration, onZoomAt }: {
  src: string; webcamSrc: string; track: CamSample[]; layout: PreviewLayout | null;
  clicks: ClickSample[]; bgUrl: string;
  cursorSprites: CursorSpriteDto[]; cursorKinds: CursorKindSample[]; cursor: CursorSettings;
  effects: EffectRegion[]; spotlightHolds: HoldSpan[]; clickfx: ClickFxSettings;
  audioSrc: string; muted: boolean; timeMs: number;
  playing: boolean; onTime: (ms: number) => void; onDuration: (ms: number) => void;
  onZoomAt: (x: number, y: number) => void;
}) {
  const screen = useRef<HTMLVideoElement>(null);
  const webcam = useRef<HTMLVideoElement>(null);
  const audio = useRef<HTMLAudioElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [err, setErr] = useState<string | null>(null);
  const playRef = useRef(playing); playRef.current = playing;
  const timeRef = useRef(timeMs); timeRef.current = timeMs;
  const onTimeRef = useRef(onTime); onTimeRef.current = onTime;
  const trackRef = useRef(track); trackRef.current = track;
  const layoutRef = useRef(layout); layoutRef.current = layout;
  const clicksRef = useRef(clicks); clicksRef.current = clicks;
  const bgImg = useRef<HTMLImageElement | null>(null);

  // Decode the export background (a data URL) once per change into an <img> the canvas draws.
  useEffect(() => {
    if (!bgUrl) { bgImg.current = null; return; }
    const img = new Image();
    img.src = bgUrl;
    bgImg.current = img;
  }, [bgUrl]);

  // Cursor: the recording's settings + type track in refs, and the sprite pack decoded into
  // <img>s by kind, so the rAF loop can draw the real cursor (matching the export).
  const kindsRef = useRef(cursorKinds); kindsRef.current = cursorKinds;
  const cursorRef = useRef(cursor); cursorRef.current = cursor;
  const effectsRef = useRef(effects); effectsRef.current = effects;
  const holdsRef = useRef(spotlightHolds); holdsRef.current = spotlightHolds;
  const clickfxRef = useRef(clickfx); clickfxRef.current = clickfx;
  const spritesRef = useRef({ sprites: new Map<string, HTMLImageElement>(), hots: new Map<string, [number, number]>(), canvasH: new Map<string, number>() });
  const trailRef = useRef<[number, number][]>([]);
  useEffect(() => {
    const sprites = new Map<string, HTMLImageElement>(), hots = new Map<string, [number, number]>(), canvasH = new Map<string, number>();
    for (const s of cursorSprites) { const img = new Image(); img.src = s.url; sprites.set(s.kind, img); hots.set(s.kind, s.hot); canvasH.set(s.kind, s.canvas_h); }
    spritesRef.current = { sprites, hots, canvasH };
  }, [cursorSprites]);

  useEffect(() => {
    const sv = screen.current, wv = webcam.current, av = audio.current;
    if (playing) { sv?.play().catch(() => {}); wv?.play().catch(() => {}); av?.play().catch(() => {}); }
    else { sv?.pause(); wv?.pause(); av?.pause(); }
  }, [playing, src]);
  // The preview audio plays from a separate <audio> (the proxy video is silent); mute toggle.
  useEffect(() => { if (audio.current) audio.current.muted = muted; }, [muted, audioSrc]);

  // Seek the videos to match the scrubbed time, but ONLY when paused. Guard on the live ref,
  // not the captured `playing`: this effect re-runs on every rAF `setTimeMs` during playback,
  // and a stale `playing===false` closure here would seek the video backward mid-play (the
  // playhead "loops" from the middle). `playRef.current` is always current, so it can't.
  useEffect(() => {
    if (playRef.current) return;
    const sv = screen.current, wv = webcam.current, av = audio.current;
    if (sv && Math.abs(sv.currentTime * 1000 - timeMs) > 40) sv.currentTime = Math.max(0, timeMs / 1000);
    if (wv && Math.abs(wv.currentTime * 1000 - timeMs) > 40) wv.currentTime = Math.max(0, timeMs / 1000);
    if (av && Math.abs(av.currentTime * 1000 - timeMs) > 40) av.currentTime = Math.max(0, timeMs / 1000);
  }, [timeMs, playing]);

  // One rAF loop: read the time (the video while playing, else the prop), keep the webcam
  // roughly synced, and composite the frame onto the canvas.
  useEffect(() => {
    let raf = 0;
    const tick = () => {
      const sv = screen.current, c = canvas.current;
      if (sv && c) {
        const t = playRef.current ? sv.currentTime * 1000 : timeRef.current;
        if (playRef.current) {
          onTimeRef.current(t);
          const wv = webcam.current, av = audio.current;
          if (wv && Math.abs(wv.currentTime - sv.currentTime) > 0.15) wv.currentTime = sv.currentTime;
          if (av && Math.abs(av.currentTime - sv.currentTime) > 0.18) av.currentTime = sv.currentTime;
        }
        const ctx = c.getContext("2d");
        if (ctx) {
          const sp = spritesRef.current, cs = cursorRef.current, cf = clickfxRef.current;
          const cur = { style: cs.style, size: cs.size, clickBounce: cs.click_bounce, bounceIntensity: cs.bounce_intensity,
            motionBlur: cs.motion_blur, kinds: kindsRef.current, sprites: sp.sprites, hots: sp.hots, canvasH: sp.canvasH, recent: trailRef.current };
          const spot = cf.enabled
            ? { effects: effectsRef.current, holds: holdsRef.current, on: cf.spotlight, params: { dim: cf.spotlight_dim, radius: cf.spotlight_radius, feather: cf.spotlight_feather } }
            : null;
          drawPreview(ctx, c.width, c.height, sv, webcam.current, camAt(trackRef.current, t),
            layoutRef.current, bgImg.current, clicksRef.current, t, cur, spot);
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);

  // Click the preview to add a zoom focused on that point: inverse-map the click through the
  // current zoom crop + screen rect to a 0..1 screen-content fraction (the zoom target).
  const onCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const c = canvas.current, sv = screen.current; if (!c || !sv) return;
    const vw = sv.videoWidth, vh = sv.videoHeight; if (vw <= 0 || vh <= 0) return;
    const rect = c.getBoundingClientRect();
    const cx = ((e.clientX - rect.left) / rect.width) * c.width;
    const cy = ((e.clientY - rect.top) / rect.height) * c.height;
    const lay = layoutRef.current, pad = Math.min(c.width, c.height) * 0.045;
    const dx = lay ? lay.screen[0] * c.width : pad, dy = lay ? lay.screen[1] * c.height : pad;
    const dw = lay ? lay.screen[2] * c.width : c.width - 2 * pad, dh = lay ? lay.screen[3] * c.height : c.height - 2 * pad;
    if (cx < dx || cx > dx + dw || cy < dy || cy > dy + dh) return; // only inside the screen
    const cam = camAt(trackRef.current, timeRef.current);
    const s = Math.max(1, cam.scale), sw = vw / s, sh = vh / s;
    const sx = Math.max(0, Math.min(vw - sw, cam.cx * vw - sw / 2));
    const sy = Math.max(0, Math.min(vh - sh, cam.cy * vh - sh / 2));
    onZoomAt((sx + ((cx - dx) / dw) * sw) / vw, (sy + ((cy - dy) / dh) * sh) / vh);
  };

  return (
    <div className="e-stagewrap">
      <div className="e-ftool">
        <button title="Aspect ratio"><IconAspectRatio size={18} /></button>
        <button title="Cursor"><IconClick size={18} /></button>
        <button title="Captions"><IconTypography size={18} /></button>
        <button title="3D camera"><IconVideo size={18} /></button>
      </div>
      <div className="e-stage">
        {!src && !err && <div className="e-stage-empty"><Spin size={20} /><span>Preparing preview</span></div>}
        <canvas ref={canvas} className="e-canvas" width={1280} height={720} onClick={onCanvasClick}
          title="Click to add a zoom here" style={{ display: src ? "block" : "none", cursor: "zoom-in" }} />
        {src && (
          <video ref={screen} src={src} muted playsInline preload="auto" style={HIDDEN}
            onLoadedData={() => setErr(null)}
            onLoadedMetadata={(e) => {
              const v = e.currentTarget; const d = v.duration;
              if (isFinite(d) && d > 0) onDuration(Math.round(d * 1000));
              v.currentTime = Math.max(0, timeRef.current / 1000); // restore position across the raw->proxy swap
              if (playRef.current) v.play().catch(() => {});
            }}
            onError={(e) => setErr(MEDIA_ERR[e.currentTarget.error?.code ?? 0] || "load failed")} />
        )}
        {webcamSrc && <video ref={webcam} src={webcamSrc} muted playsInline preload="auto" style={HIDDEN} />}
        {audioSrc && <audio ref={audio} src={audioSrc} preload="auto" />}
        {err && <div className="e-stage-empty" style={{ position: "absolute", inset: 0 }}>Preview unavailable - {err}</div>}
      </div>
    </div>
  );
}
