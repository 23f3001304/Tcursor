import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { AnimatePresence, motion } from "motion/react";
import { placeBeside, portalHost, type Placement } from "./popoverPlace";

export const TOOLTIP_DELAY_MS = 350;

export function Tooltip({
  label,
  children,
  className = "",
}: {
  label: string;
  children: ReactNode;
  className?: string;
}) {
  const [show, setShow] = useState(false);
  const [at, setAt] = useState<Placement | null>(null);
  const [host, setHost] = useState<Element | null>(null);
  const wrap = useRef<HTMLSpanElement>(null);
  const tip = useRef<HTMLSpanElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const clear = () => {
    if (timer.current !== null) {
      clearTimeout(timer.current);
      timer.current = null;
    }
  };
  const reveal = () => {
    setAt(null);
    setShow(true);
  };
  const hide = () => {
    clear();
    setShow(false);
  };
  useEffect(() => clear, []);
  useLayoutEffect(() => {
    setHost(portalHost(wrap.current));
  }, []);

  useLayoutEffect(() => {
    if (!show || at || !wrap.current || !tip.current) return;
    const box = tip.current.getBoundingClientRect();
    setAt(
      placeBeside(
        wrap.current.getBoundingClientRect(),
        box.width,
        box.height,
        window.innerWidth,
        window.innerHeight,
      ),
    );
  }, [show, at]);

  return (
    <span
      ref={wrap}
      className={`e-tipwrap ${className}`.trim()}
      onMouseEnter={() => {
        clear();
        timer.current = setTimeout(reveal, TOOLTIP_DELAY_MS);
      }}
      onMouseLeave={hide}
      onPointerDown={hide}
      onFocus={() => {
        clear();
        reveal();
      }}
      onBlur={hide}
    >
      {children}
      {host &&
        createPortal(
          <AnimatePresence>
            {show && (
              <motion.span
                ref={tip}
                className="e-tip"
                role="tooltip"
                style={{ left: at?.left ?? 0, top: at?.top ?? 0, visibility: at ? "visible" : "hidden" }}
                initial={{ opacity: 0, x: at?.flipped ? 4 : -4 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: at?.flipped ? 4 : -4 }}
                transition={{ type: "tween", duration: 0.12, ease: [0.4, 0, 0.2, 1] }}
              >
                {label}
              </motion.span>
            )}
          </AnimatePresence>,
          host,
        )}
    </span>
  );
}
