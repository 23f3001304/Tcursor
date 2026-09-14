import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { AnimatePresence, motion } from "motion/react";
import { getSettings } from "../../lib/ipc";
import { useReducedMotion } from "../../lib/wave/ui/useReducedMotion";
import { interfaceEffectsOn, setInterfaceEffects } from "./effectsFlag";
import { RIPPLE_FROM, RIPPLE_MS, RIPPLE_TO, dropRipple, pushRipple, rippleTone, suppressesRipple, type Ripple } from "./ripples";
import "./effects.css";

/** The editor's click ripple, as ONE fixed overlay for the whole window rather than a layer per
 *  control: a thin ring blooms from 8px to 56px under every pointerdown in the chrome and is gone
 *  in 320ms. One element, one listener, transforms and opacity only, `pointer-events: none` - it
 *  can neither steal a click nor shift a layout, which is the entire design constraint.
 *
 *  Mounted from `App.tsx` beside `<Editor>` (never from inside the editor tree, which another
 *  agent owns) but PORTALLED into the live `.editor` element, for two reasons that both matter:
 *  the `--e-*` palette tokens are declared on `.editor` and only inherit to its descendants, and
 *  `.e-modal-scrim` is `z-index: 100` INSIDE `.editor`'s stacking context - a sibling of `.editor`
 *  could not sit under it at any z-index. As a child at `z-index: 99` the overlay clears the
 *  panels, the timeline and the director layers (all < 51) and still passes under a modal.
 *
 *  `prefers-reduced-motion` keeps the ripple but drops the growth: it appears at full size and
 *  fades. The feedback is the point; the bloom is the flourish. */
export function InterfaceEffects() {
  const [on, setOn] = useState(interfaceEffectsOn);
  const [host, setHost] = useState<Element | null>(null);
  const [ripples, setRipples] = useState<Ripple[]>([]);
  const nextId = useRef(0);
  const reduced = useReducedMotion();

  // The settings read. On mount, and again on every window focus - Preferences is a separate
  // surface, so focus coming back to this window is exactly when a flip over there becomes
  // visible here, with no event to subscribe to and no reload. `!== false` so an older backend
  // that does not send the field yet still resolves to on rather than to undefined.
  useEffect(() => {
    let alive = true;
    const read = () => void getSettings()
      .then((s) => { if (alive) { const v = s.ui.interface_effects !== false; setInterfaceEffects(v); setOn(v); } })
      .catch(() => { /* keep whatever the flag already says */ });
    read();
    window.addEventListener("focus", read);
    return () => { alive = false; window.removeEventListener("focus", read); };
  }, []);

  // The portal target, resolved after the first commit - `Editor` is a sibling in the same commit,
  // so its root exists by the time effects run. Re-queried whenever the flag flips back on, which
  // covers the editor having been closed and reopened while the effects were off.
  useEffect(() => { setHost(on ? document.querySelector(".editor") : null); }, [on]);

  // The one listener. In the CAPTURE phase on purpose: the timeline's pills and the stage's
  // handles all `stopPropagation()` their own pointerdown, and a bubble-phase listener would
  // simply never hear those. Capture hears everything, and `data-ui-fx="off"` - not propagation -
  // is what decides which surfaces stay silent.
  useEffect(() => {
    if (!on) { setRipples([]); return; }
    const onDown = (e: PointerEvent) => {
      const target = e.target instanceof Element ? e.target : null;
      if (suppressesRipple(target)) return;
      // Self-heal the portal target. React reuses `.editor`'s DOM node across the doc-loading
      // branch, so in practice it never detaches - but a detached host would silently swallow
      // every ripple, and re-querying only when `isConnected` is false costs nothing (the same
      // reference back means React bails out of this setState).
      setHost((h) => (h?.isConnected ? h : document.querySelector(".editor")));
      setRipples((list) => pushRipple(list, { id: ++nextId.current, x: e.clientX, y: e.clientY, tone: rippleTone(target) }));
    };
    const opts = { capture: true, passive: true } as const;
    window.addEventListener("pointerdown", onDown, opts);
    return () => window.removeEventListener("pointerdown", onDown, opts);
  }, [on]);

  if (!on || !host) return null;
  return createPortal(
    <div className="e-ui-fx" aria-hidden>
      {/* AnimatePresence earns its place on the CAP, not on the normal life cycle: a ripple that
          finishes its own fade removes itself below, but the seventh ripple evicts the oldest
          mid-bloom, and without an exit that one would vanish as a hard pop. */}
      <AnimatePresence>
        {ripples.map((r) => (
          <motion.span key={r.id} className={`e-ui-ripple${r.tone === "accent" ? " accent" : ""}`}
            style={{ left: r.x, top: r.y }}
            initial={{ scale: reduced ? 1 : RIPPLE_FROM / RIPPLE_TO, opacity: reduced ? 0.55 : 0.9 }}
            animate={{ scale: 1, opacity: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: RIPPLE_MS / 1000, ease: [0.2, 0, 0.2, 1] }}
            onAnimationComplete={() => setRipples((list) => dropRipple(list, r.id))} />
        ))}
      </AnimatePresence>
    </div>,
    host,
  );
}
