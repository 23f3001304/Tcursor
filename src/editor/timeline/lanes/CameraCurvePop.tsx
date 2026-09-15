import { useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { PRESETS, presetOf, presetPatch } from "../../motion/presets";
import { PresetRow } from "../../motion/PresetRow";
import {
  placeStackedCentred,
  portalHost,
  type Placement,
  type Rect,
} from "../../controls/surfaces/popoverPlace";

const POP_W = 312;

export function CameraCurvePop({
  anchor,
  easing,
  onPick,
  onDismiss,
}: {
  anchor: Rect;
  easing: string;
  onPick: (easing: string) => void;
  onDismiss: () => void;
}) {
  const [at, setAt] = useState<Placement | null>(null);
  const [host, setHost] = useState<Element | null>(null);
  const box = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    setHost(portalHost(box.current));
  }, []);
  useLayoutEffect(() => {
    if (at || !box.current) return;
    setAt(
      placeStackedCentred(
        anchor,
        POP_W,
        box.current.offsetHeight,
        window.innerWidth,
        window.innerHeight,
        6,
        true,
      ),
    );
  }, [anchor, at]);

  useLayoutEffect(() => {
    window.addEventListener("scroll", onDismiss, true);
    window.addEventListener("resize", onDismiss);
    return () => {
      window.removeEventListener("scroll", onDismiss, true);
      window.removeEventListener("resize", onDismiss);
    };
  }, [onDismiss]);

  const pop = (
    <div
      ref={box}
      className="e-campop"
      onPointerDown={(e) => e.stopPropagation()}
      style={{ left: at?.left ?? 0, top: at?.top ?? 0, visibility: at ? "visible" : "hidden" }}
    >
      <PresetRow
        presets={PRESETS}
        value={presetOf(easing, null)}
        onPick={(id) => {
          if (id !== "custom") onPick(presetPatch(id).easing);
        }}
      />
    </div>
  );
  return host ? createPortal(pop, host) : pop;
}
