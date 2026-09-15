import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { AnimatePresence, motion } from "motion/react";
import { getSettings } from "../../shared/ipc";
import { useReducedMotion } from "../../shared/wave/ui/useReducedMotion";
import { interfaceEffectsOn, setInterfaceEffects } from "./effectsFlag";
import {
  RIPPLE_FROM,
  RIPPLE_MS,
  RIPPLE_TO,
  dropRipple,
  pushRipple,
  rippleTone,
  suppressesRipple,
  type Ripple,
} from "./ripples";
import "./effects.css";

export function InterfaceEffects() {
  const [on, setOn] = useState(interfaceEffectsOn);
  const [host, setHost] = useState<Element | null>(null);
  const [ripples, setRipples] = useState<Ripple[]>([]);
  const nextId = useRef(0);
  const reduced = useReducedMotion();

  useEffect(() => {
    let alive = true;
    const read = () =>
      void getSettings()
        .then((s) => {
          if (alive) {
            const v = s.ui.interface_effects !== false;
            setInterfaceEffects(v);
            setOn(v);
          }
        })
        .catch(() => {});
    read();
    window.addEventListener("focus", read);
    return () => {
      alive = false;
      window.removeEventListener("focus", read);
    };
  }, []);

  useEffect(() => {
    setHost(on ? document.querySelector(".editor") : null);
  }, [on]);

  useEffect(() => {
    if (!on) {
      setRipples([]);
      return;
    }
    const onDown = (e: PointerEvent) => {
      const target = e.target instanceof Element ? e.target : null;
      if (suppressesRipple(target)) return;
      setHost((h) => (h?.isConnected ? h : document.querySelector(".editor")));
      setRipples((list) =>
        pushRipple(list, { id: ++nextId.current, x: e.clientX, y: e.clientY, tone: rippleTone(target) }),
      );
    };
    const opts = { capture: true, passive: true } as const;
    window.addEventListener("pointerdown", onDown, opts);
    return () => window.removeEventListener("pointerdown", onDown, opts);
  }, [on]);

  if (!on || !host) return null;
  return createPortal(
    <div className="e-ui-fx" aria-hidden>
      <AnimatePresence>
        {ripples.map((r) => (
          <motion.span
            key={r.id}
            className={`e-ui-ripple${r.tone === "accent" ? " accent" : ""}`}
            style={{ left: r.x, top: r.y }}
            initial={{ scale: reduced ? 1 : RIPPLE_FROM / RIPPLE_TO, opacity: reduced ? 0.55 : 0.9 }}
            animate={{ scale: 1, opacity: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: RIPPLE_MS / 1000, ease: [0.2, 0, 0.2, 1] }}
            onAnimationComplete={() => setRipples((list) => dropRipple(list, r.id))}
          />
        ))}
      </AnimatePresence>
    </div>,
    host,
  );
}
