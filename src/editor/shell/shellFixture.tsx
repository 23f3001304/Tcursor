import { identityMap } from "../../lib/remap";
import type { EditDoc, Zoom } from "../../lib/edit";
import type { ShellProps } from "./slotProps";

/** Every prop `Editor.tsx` hands the shell, at rest, for the specs that mount one slot of it. The
 *  document is one zoom, which is enough for the inspectors to have something real to route to. */
const ZOOM: Zoom = { id: "z1", start_ms: 0, end_ms: 1000, target: "cursor", scale: 2, easing: "smooth", zoom_in_ms: 300, zoom_out_ms: 300, layer: 0 };
export const DOC = { zooms: [ZOOM], effects: [], layout: [], camera_moves: [], cuts: [], speed: [], aspect: "source", trim: { in_ms: 0, out_ms: 5000 },
  settings: { ai_model: "", cursor: {}, clickfx: {}, zoom: {}, appearance: {}, ui: {} } } as unknown as EditDoc;
const noop = () => {};

/** Every prop `Editor.tsx` hands the shell, at rest. */
export function shellProps(over: Partial<ShellProps>): ShellProps {
  return {
    folder: "C:/p", doc: DOC, sel: null, setSel: noop, onSel: noop, tab: "ai", setTab: noop, dur: 5000,
    timeMs: 0, timeMsRef: { current: 0 }, playing: false, muted: false, volume: 100, setVolume: noop,
    quality: 720, cycleQuality: noop, onMuteToggle: noop, exporting: false, running: false, trimmed: false,
    aimMode: false, aimPoint: null, setAimOn: noop, moveMode: false, onMoveMode: noop, arrangeSeg: null,
    arrangeOn: false, onArrange: noop, camDraftRef: { current: null }, srcUrl: "", webcamSrc: "", audioUrl: "",
    bg: { url: "", assetUrl: "", assetPath: "", kind: "mesh", dim: 0 }, map: identityMap(5000), outDoc: DOC,
    range: null, setRange: noop, onDetectSilences: noop, track: [], layout: null, layoutPresets: null, clicks: [], cursorSpr: null, cursorKnd: [],
    cursorLyr: null, osCursor: true, thumbs: [], waves: { system: "", mic: "" }, wavesReady: false,
    hasWebcam: false, aspectLocked: false, onTime: noop, onSeek: noop, onPlayToggle: noop, onAspect: noop,
    onDuration: noop, zoomAt: noop, aimAt: noop, applyOp: async () => null, saveDocSettings: noop,
    retryMedia: noop, addZoom: noop, addSpotlight: noop, addCameraMove: noop, onRun: noop, onAutoModel: noop,
    aiError: null, aiLog: [], aiProgress: null, aiPlanning: false, pointerRef: { current: null },
    onCancelRun: noop, toast: null, dismissToast: noop, onTrimIn: noop, onTrimOut: noop, onResetTrim: noop,
    modalOpen: false,
    ...over,
  };
}
