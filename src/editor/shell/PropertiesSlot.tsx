import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { ZoomInspector } from "../inspectors/ZoomInspector";
import { EffectInspector } from "../inspectors/EffectInspector";
import { MaskInspector } from "../inspectors/MaskInspector";
import { LayoutInspector } from "../inspectors/LayoutInspector";
import { CameraMoveInspector } from "../inspectors/CameraMoveInspector";
import { CutInspector } from "../inspectors/CutInspector";
import { SpeedInspector } from "../inspectors/SpeedInspector";
import { CaptionInspector } from "../inspectors/CaptionInspector";
import { TextInspector } from "../inspectors/TextInspector";
import { ClipInspector } from "../inspectors/ClipInspector";
import type {
  CameraMove,
  Caption,
  Clip,
  Cut,
  EditDoc,
  EffectRegion,
  LayoutSeg,
  Speed,
  TextItem,
  Zoom,
} from "../../shared/edit";
import { isMask } from "../../shared/edit";
import type { SlotProps } from "./slotProps";
import "../inspectors/inspectors.css";

const SWAP_TWEEN = { type: "tween" as const, duration: 0.16, ease: [0.4, 0, 0.2, 1] as const };

export type SelectedClip =
  | { kind: "zoom"; zoom: Zoom }
  | { kind: "fx"; effect: EffectRegion }
  | { kind: "mask"; effect: EffectRegion }
  | { kind: "layout"; layout: LayoutSeg }
  | { kind: "cam"; move: CameraMove }
  | { kind: "cut"; cut: Cut }
  | { kind: "speed"; speed: Speed }
  | { kind: "caption"; caption: Caption }
  | { kind: "text"; text: TextItem }
  | { kind: "clip"; clip: Clip };

export function selectedClip(doc: EditDoc, sel: string | null): SelectedClip | null {
  if (!sel) return null;
  const zoom = doc.zooms.find((z) => z.id === sel);
  if (zoom) return { kind: "zoom", zoom };
  const effect = doc.effects.find((e) => e.id === sel);
  if (effect) return isMask(effect) ? { kind: "mask", effect } : { kind: "fx", effect };
  const layout = doc.layout.find((l) => l.id === sel);
  if (layout) return { kind: "layout", layout };
  const move = doc.camera_moves.find((m) => m.id === sel);
  if (move) return { kind: "cam", move };
  const cut = doc.cuts.find((c) => c.id === sel);
  if (cut) return { kind: "cut", cut };
  const speed = doc.speed.find((s) => s.id === sel);
  if (speed) return { kind: "speed", speed };
  const caption = doc.captions?.find((c) => c.id === sel);
  if (caption) return { kind: "caption", caption };
  const text = doc.texts?.find((t) => t.id === sel);
  if (text) return { kind: "text", text };
  const clip = doc.clips.find((c) => c.id === sel);
  if (clip) return { kind: "clip", clip };
  return null;
}

export function PropertiesSlot({ p }: { p: SlotProps }) {
  const still = useReducedMotion();
  const { doc } = p;
  const hit = selectedClip(doc, p.sel);
  const close = () => p.setSel(null);
  return (
    <AnimatePresence mode="popLayout" initial={false}>
      {hit && (
        <motion.div
          key={p.sel}
          className="e-panel-slot"
          initial={still ? false : { opacity: 0, x: -8 }}
          animate={{ opacity: 1, x: 0 }}
          exit={still ? { opacity: 0 } : { opacity: 0, x: -8 }}
          transition={still ? { duration: 0 } : SWAP_TWEEN}
        >
          {hit.kind === "zoom" ? (
            <ZoomInspector
              zoom={hit.zoom}
              zooms={doc.zooms}
              dur={p.dur}
              onApply={p.applyOp}
              onClose={close}
              aimMode={p.aimMode}
              moveMode={p.moveMode}
              onAimMode={p.setAimOn}
              timeMsRef={p.timeMsRef}
              onSeek={p.onSeek}
            />
          ) : hit.kind === "fx" ? (
            <EffectInspector
              effect={hit.effect}
              dur={p.dur}
              settings={doc.settings}
              onApply={p.applyOp}
              onClose={close}
              onDimCamera={(v) =>
                p.saveDocSettings({
                  ...doc.settings,
                  clickfx: { ...doc.settings.clickfx, spotlight_dim_camera: v },
                })
              }
            />
          ) : hit.kind === "mask" ? (
            <MaskInspector
              effect={hit.effect}
              dur={p.dur}
              settings={doc.settings}
              onApply={p.applyOp}
              onClose={close}
            />
          ) : hit.kind === "layout" ? (
            <LayoutInspector
              seg={hit.layout}
              segs={doc.layout}
              dur={p.dur}
              onApply={p.applyOp}
              onClose={close}
              presets={p.layoutPresets}
              arrangeOn={p.arrangeOn}
              onArrange={p.onArrange}
            />
          ) : hit.kind === "cam" ? (
            <CameraMoveInspector
              move={hit.move}
              moves={doc.camera_moves}
              dur={p.dur}
              onApply={p.applyOp}
              onClose={close}
            />
          ) : hit.kind === "cut" ? (
            <CutInspector cut={hit.cut} dur={p.dur} onApply={p.applyOp} onClose={close} />
          ) : hit.kind === "caption" ? (
            <CaptionInspector
              caption={hit.caption}
              captions={doc.captions}
              dur={p.dur}
              timeMs={p.timeMs}
              onApply={p.applyOp}
              onClose={close}
            />
          ) : hit.kind === "text" ? (
            <TextInspector item={hit.text} dur={p.dur} onApply={p.applyOp} onClose={close} />
          ) : hit.kind === "clip" ? (
            <ClipInspector
              clip={hit.clip}
              clips={doc.clips}
              map={p.map}
              dur={p.dur}
              motionEasing={doc.settings.motion.easing}
              onApply={p.applyOp}
              onClose={close}
            />
          ) : (
            <SpeedInspector speed={hit.speed} dur={p.dur} onApply={p.applyOp} onClose={close} />
          )}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
