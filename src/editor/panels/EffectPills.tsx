import {
  IconZoomIn,
  IconBulb,
  IconAspectRatio,
  IconVideo,
  IconBlur,
  IconGridDots,
  IconFocus2,
  IconTypography,
  IconLayoutBottombar,
  IconNumbers,
  IconMessage2,
} from "@tabler/icons-react";
import type { MaskKind, TextKind } from "../../shared/edit";
import { TEXT_PILLS } from "./textStyles";

export interface PillActions {
  onAddZoom: () => void;
  onAddSpotlight: () => void;
  onAddMask: (kind: MaskKind) => void;
  onAddLayout: () => void;
  onAddCameraMove: () => void;
  onAddText: (kind: TextKind) => void;
}

interface Pill {
  type: string;
  cls: string;
  Icon: typeof IconZoomIn;
  name: string;
  hint: string;
  run: (a: PillActions) => void;
}

const TEXT_ICONS: Record<TextKind, typeof IconZoomIn> = {
  title: IconTypography,
  lower_third: IconLayoutBottombar,
  stat: IconNumbers,
  callout: IconMessage2,
};

export const PILLS: Pill[] = [
  {
    type: "layout",
    cls: "e-layblk",
    Icon: IconAspectRatio,
    name: "Layout Segment",
    hint: "Switch the frame layout",
    run: (a) => a.onAddLayout(),
  },
  {
    type: "zoom",
    cls: "e-zblk",
    Icon: IconZoomIn,
    name: "Zoom Region",
    hint: "Push in on a click or region",
    run: (a) => a.onAddZoom(),
  },
  {
    type: "spotlight",
    cls: "e-fxblk",
    Icon: IconBulb,
    name: "Spotlight Highlight",
    hint: "Dim everything but the focus",
    run: (a) => a.onAddSpotlight(),
  },
  {
    type: "blur",
    cls: "e-fxblk",
    Icon: IconBlur,
    name: "Blur Mask",
    hint: "Hide a region behind a blur",
    run: (a) => a.onAddMask("blur"),
  },
  {
    type: "pixelate",
    cls: "e-fxblk",
    Icon: IconGridDots,
    name: "Pixelate Mask",
    hint: "Hide a region behind big pixels",
    run: (a) => a.onAddMask("pixelate"),
  },
  {
    type: "highlight",
    cls: "e-fxblk",
    Icon: IconFocus2,
    name: "Highlight Mask",
    hint: "Dim everything outside a rectangle",
    run: (a) => a.onAddMask("highlight"),
  },
  {
    type: "cammove",
    cls: "e-camkfpill",
    Icon: IconVideo,
    name: "Camera Move",
    hint: "Keyframe the webcam",
    run: (a) => a.onAddCameraMove(),
  },
  ...TEXT_PILLS.map((p) => ({
    type: `text:${p.kind}`,
    cls: "e-textblk",
    Icon: TEXT_ICONS[p.kind],
    name: p.name,
    hint: p.hint,
    run: (a: PillActions) => a.onAddText(p.kind),
  })),
];

const handleDragStart = (e: React.DragEvent, type: string) => {
  e.dataTransfer.setData("text/plain", type);
  e.dataTransfer.effectAllowed = "copy";
};

export function EffectPills(a: PillActions) {
  return (
    <div className="e-grp">
      <span className="e-sechead">Insert timeline elements</span>
      <div className="e-libgrid">
        {PILLS.map((p) => (
          <div
            key={p.type}
            draggable
            onDragStart={(e) => handleDragStart(e, p.type)}
            onClick={() => p.run(a)}
            className={`${p.cls} e-libpill`}
          >
            <p.Icon size={16} className="e-libicon" />
            <span className="e-libtext">
              <span className="e-libname">{p.name}</span>
              <span className="e-libhint">{p.hint}</span>
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
