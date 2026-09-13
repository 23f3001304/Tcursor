import type { ComponentType } from "react";
import { motion } from "motion/react";
import { IconAspectRatio, IconClick, IconTypography, IconVideo } from "@tabler/icons-react";
import type { Tab } from "../shell/panelTabs";
import type { Aspect } from "../../lib/edit";
import { ASPECT_ORDER, ASPECT_LABEL } from "./Transport";

const TAP = { whileTap: { scale: 0.92 }, transition: { type: "tween" as const, duration: 0.1 } };

/** Floating stage overlay: quick shortcuts to the panels/controls that most shape what's on
 *  screen right now. Investigated (2026-09-01 sweep): these buttons carried NO props and drove
 *  nothing - the old "on" state was a local `useState` that just recolored the icon and did
 *  nothing else, which is exactly why the audit read them as "unlabeled toggles with no visible
 *  effect." There's no live "hide this layer" flag behind Cursor/Captions/Camera to wire to
 *  honestly (captions in particular only bake in at export, never in the live preview - see
 *  CaptionsPanel), so instead of faking layer toggles this jumps straight to the real, already-
 *  wired control: Aspect ratio cycles the exact same sequence as Transport's chip, and Cursor/
 *  Captions/Camera open their Rail panel.
 *
 *  Two DIFFERENT kinds of button, deliberately styled apart (fix round 1, controller ruling on
 *  the toggle-vs-navigation question):
 *  - **Aspect** is a real toggle of a live stage property - the canvas itself reshapes when
 *    clicked. It keeps the strong, latched `.on` look (`--e-primary` red accent, `.e-tbtn.on`'s
 *    own language) and a state-sentence tooltip ("Aspect ratio: 16:9, click to switch to 9:16").
 *  - **Cursor / Captions / Camera** are navigation, not toggles - clicking one doesn't change
 *    anything about the STAGE, it changes which panel is showing. Their `.nav-on` look only
 *    ever MIRRORS the workspace rail's active highlight (`.e-wsric.on`'s soft neutral tint, not a
 *    colored accent) rather than latching its own independent "engaged" state, and their tooltip
 *    is a constant "Open the {X} panel" regardless of whether that panel happens to already be
 *    open - clicking an already-open one is a harmless no-op, not a toggle-off. A thin divider
 *    (`.e-ftool-div`, same `--e-border2` token Transport's own `.e-tdiv` uses) separates the two
 *    groups so the row reads as "one live control, then three shortcuts", not four uniform buttons. */
export function StageToolbar({ tab, onTab, aspect, onAspect, aspectLocked }: {
  tab: Tab; onTab: (t: Tab) => void;
  aspect: Aspect; onAspect: (a: Aspect) => void;
  /** Mirrors Transport's own `locked` (exporting, or nothing loaded yet) - the two aspect
   *  controls must agree on when cycling is unsafe, not just on the cycle order. */
  aspectLocked: boolean;
}) {
  const nextAspect = ASPECT_ORDER[(ASPECT_ORDER.indexOf(aspect) + 1) % ASPECT_ORDER.length];
  const cycleAspect = () => onAspect(nextAspect);

  const NAV: { id: Tab; icon: ComponentType<{ size?: number }>; label: string }[] = [
    { id: "cursor", icon: IconClick, label: "Cursor" },
    { id: "captions", icon: IconTypography, label: "Captions" },
    { id: "camera", icon: IconVideo, label: "Camera" },
  ];

  return (
    <div className="e-ftool">
      <motion.button className={aspect !== "source" ? "on" : ""} disabled={aspectLocked} onClick={cycleAspect}
        title={`Aspect ratio: ${ASPECT_LABEL[aspect]} - click to switch to ${ASPECT_LABEL[nextAspect]}`}
        aria-label="Aspect ratio" aria-pressed={aspect !== "source"} {...TAP}>
        <IconAspectRatio size={16} />
      </motion.button>
      <div className="e-ftool-div" />
      {NAV.map(({ id, icon: Icon, label }) => (
        <motion.button key={id} className={tab === id ? "nav-on" : ""} onClick={() => onTab(id)}
          title={`Open the ${label} panel`} aria-label={label} aria-current={tab === id || undefined} {...TAP}>
          <Icon size={16} />
        </motion.button>
      ))}
    </div>
  );
}
