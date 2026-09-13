import { motion } from "motion/react";
import { PANEL_TABS, type Tab } from "./panelTabs";

/** The one press spring the whole app's quiet chrome uses (design/premium-pass D6) - scale .96,
 *  no hover bump (unlike Transport's TAP_SPRING), since a rail tab's active-pill glide is already
 *  the primary hover signal. */
const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

/** Left icon rail: one button per editor tab. The active pill is a single shared motion.span
 *  (layoutId "rail-active") that glides between tools; the icon color still snaps (.on),
 *  and "ai" keeps the violet tint. The classic composition, back by the owner's call after the
 *  area/workspace shell was vetoed (that shell is archived on `archive/m1a-shell`). */
export function Rail({ tab, onTab }: { tab: Tab; onTab: (t: Tab) => void }) {
  return (
    <div className="e-rail">
      {PANEL_TABS.map(({ id, icon: Icon, label }) => (
        <motion.button key={id} className={`e-ric${id === "ai" ? " ai" : ""}${tab === id ? " on" : ""}`}
          title={label} aria-label={label} onClick={() => onTab(id)}
          whileTap={PRESS_TAP} transition={PRESS_SPRING}>
          {tab === id && (
            <motion.span className="e-ric-hl" layoutId="rail-active"
              transition={{ type: "tween", duration: 0.18, ease: [0.4, 0, 0.2, 1] }} />
          )}
          <Icon size={20} />
        </motion.button>
      ))}
    </div>
  );
}
