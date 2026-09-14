import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { AnimatePresence, motion } from "motion/react";
import { placeBeside, portalHost, type Placement } from "./popoverPlace";

/** Long enough that sweeping across the rail never lights a trail of labels, short enough that
 *  stopping on a button answers "what is this?" before the native `title` would. */
export const TOOLTIP_DELAY_MS = 350;

/** A hover label for an icon-only control: the child renders as-is, and after
 *  `TOOLTIP_DELAY_MS` of hover a small label fades in beside it (`.e-tip`, placed to the right by
 *  default - see `stage.css`). Keyboard focus shows it immediately, since a focused control has
 *  already been deliberately reached. The caller keeps its own `title` attribute: this is the fast,
 *  styled path, the native tooltip is the accessible fallback.
 *
 *  The label is PORTALLED out of the wrapper (`portalHost`) and placed in viewport coordinates, so
 *  no `overflow` ancestor between the control and the editor root can clip it - it was being cut
 *  off inside a horizontally scrolling tile strip, and the rail, the panels and the inspector
 *  column are all overflow containers too. */
export function Tooltip({ label, children, className = "" }: { label: string; children: ReactNode; className?: string }) {
  const [show, setShow] = useState(false);
  // `null` while the label has been mounted but not yet measured - it renders hidden for that one
  // pass, so it never flashes at the window's top-left corner before landing.
  const [at, setAt] = useState<Placement | null>(null);
  // Resolved once, after mount: `portalHost` needs a mounted node to walk up from, and a container
  // that changed identity between renders would tear the portal down and rebuild it.
  const [host, setHost] = useState<Element | null>(null);
  const wrap = useRef<HTMLSpanElement>(null);
  const tip = useRef<HTMLSpanElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const clear = () => { if (timer.current !== null) { clearTimeout(timer.current); timer.current = null; } };
  // The placement is dropped when the label is SHOWN, not when it is hidden: clearing it on hide
  // would blank the label for its own exit fade, which is the one moment it has to still be there.
  const reveal = () => { setAt(null); setShow(true); };
  const hide = () => { clear(); setShow(false); };
  // The pending timer must die with the component - a rail button can unmount mid-hover.
  useEffect(() => clear, []);
  useLayoutEffect(() => { setHost(portalHost(wrap.current)); }, []);

  // Layout effect, not a plain effect: measuring and placing both happen before the browser paints,
  // which is what makes the hidden first pass invisible rather than a one-frame flicker. Guarded on
  // `at` being unset so the `setAt` below cannot loop.
  useLayoutEffect(() => {
    if (!show || at || !wrap.current || !tip.current) return;
    const box = tip.current.getBoundingClientRect();
    setAt(placeBeside(wrap.current.getBoundingClientRect(), box.width, box.height,
      window.innerWidth, window.innerHeight));
  }, [show, at]);

  return (
    <span ref={wrap} className={`e-tipwrap ${className}`.trim()}
      onMouseEnter={() => { clear(); timer.current = setTimeout(reveal, TOOLTIP_DELAY_MS); }}
      onMouseLeave={hide} onPointerDown={hide} onFocus={() => { clear(); reveal(); }} onBlur={hide}>
      {children}
      {host && createPortal(
        <AnimatePresence>
          {show && (
            <motion.span ref={tip} className="e-tip" role="tooltip"
              style={{ left: at?.left ?? 0, top: at?.top ?? 0, visibility: at ? "visible" : "hidden" }}
              initial={{ opacity: 0, x: at?.flipped ? 4 : -4 }} animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: at?.flipped ? 4 : -4 }}
              transition={{ type: "tween", duration: 0.12, ease: [0.4, 0, 0.2, 1] }}>
              {label}
            </motion.span>
          )}
        </AnimatePresence>,
        host
      )}
    </span>
  );
}
