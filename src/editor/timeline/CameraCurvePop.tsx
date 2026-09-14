import { useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { CURVE_GLYPHS } from "./curveGlyphs";
import { placeStackedCentred, portalHost, type Placement, type Rect } from "../controls/popoverPlace";

/** `.e-campop`'s own width (timeline.css) - placement needs it before the grid has been measured. */
const POP_W = 170;

/** The Camera lane's transition-curve popover: six glyph buttons over the clicked segment, picking
 *  the easing into the later keyframe.
 *
 *  PORTALLED, because the lane it belongs to is two overflow containers deep: `.e-camrow` clips its
 *  own content and `.e-tracks` scrolls, so a popover drawn in flow was cut off on both axes - the
 *  left half vanished for a keyframe near the start of the clip, and the whole thing vanished when
 *  Camera was the FIRST lane in the stack (a doc with no zoom, FX, layout or speed clips), since it
 *  opens upward. `placeStackedCentred` keeps it centred on the segment, clamped into the window,
 *  and flips it below the lane when there is no room above. */
export function CameraCurvePop({ anchor, easing, onPick, onDismiss }: {
  /** The clicked segment's viewport rect, captured at pointerdown. */
  anchor: Rect;
  easing: string;
  onPick: (key: string) => void;
  /** Fired when a scroll or a resize moves the lane out from under the popover. */
  onDismiss: () => void;
}) {
  const [at, setAt] = useState<Placement | null>(null);
  const [host, setHost] = useState<Element | null>(null);
  const box = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => { setHost(portalHost(box.current)); }, []);
  useLayoutEffect(() => {
    if (at || !box.current) return;
    setAt(placeStackedCentred(anchor, POP_W, box.current.offsetHeight,
      window.innerWidth, window.innerHeight, 6, true));
  }, [anchor, at]);

  // Fixed coordinates are pinned to where the segment WAS, so anything that moves it closes this
  // rather than leaving it floating over an unrelated lane. Capture phase: the track stack's own
  // scroll does not bubble to the window.
  useLayoutEffect(() => {
    window.addEventListener("scroll", onDismiss, true);
    window.addEventListener("resize", onDismiss);
    return () => { window.removeEventListener("scroll", onDismiss, true); window.removeEventListener("resize", onDismiss); };
  }, [onDismiss]);

  const pop = (
    // `onPointerDown` stopPropagation: without it, every click here bubbled to `.e-tlbody`'s own
    // scrub handler (Timeline.tsx), which seeks the playhead + pauses playback AND takes pointer
    // capture - stealing the button's own `onClick` (the easing choice) half the time too (M3).
    <div ref={box} className="e-campop" onPointerDown={(e) => e.stopPropagation()}
      style={{ left: at?.left ?? 0, top: at?.top ?? 0, visibility: at ? "visible" : "hidden" }}>
      {CURVE_GLYPHS.map((c) => (
        <button key={c.key} type="button" title={c.name}
          className={`e-campop-b${easing === c.key ? " on" : ""}`} onClick={() => onPick(c.key)}>
          <svg viewBox="0 -30 100 160"><path d={c.path} fill="none"
            stroke={easing === c.key ? "var(--e-fg)" : "var(--e-mut)"} strokeWidth="8" strokeLinecap="round" /></svg>
        </button>
      ))}
    </div>
  );
  return host ? createPortal(pop, host) : pop;
}
