import { motion } from "motion/react";
import { Tooltip } from "../controls/Tooltip";
import { PANEL_TABS, type Tab } from "./panelTabs";

/** The one press spring the whole app's quiet chrome uses (design/premium-pass D6) - scale .96,
 *  no hover bump (unlike Transport's TAP_SPRING), since a rail tab's active-pill glide is already
 *  the primary hover signal. */
const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

/** Left icon rail: one button per editor tab. The active pill is a single shared motion.span
 *  (layoutId "rail-active") that glides between tools; the icon color still snaps (.on),
 *  and "ai" keeps the violet tint. The classic composition, back by the owner's call after the
 *  area/workspace shell was vetoed (that shell is archived on `archive/m1a-shell`).
 *
 *  Every button is named twice: a `Tooltip` label that fades in to the right of the rail after a
 *  350ms hover delay (the styled path), and the plain `title` underneath it (the fallback). Icons
 *  alone were the rail's standing complaint - "the left toolbar icons need tooltips and hover
 *  states" - so the resting icon is dim and hover lifts it onto `--e-raised` at full `--e-fg`.
 *
 *  `tab` is `null` while the panel column is collapsed: no tab is current, no active pill is drawn,
 *  and the rail itself is still there - it is the way back in. Whether a click opens or closes is
 *  `onTab`'s business (`nextTab`, `panelState.ts`), not this component's; the rail just reports
 *  which icon was pressed. */
export function Rail({ tab, onTab }: { tab: Tab | null; onTab: (t: Tab) => void }) {
  return (
    <div className="e-rail">
      {PANEL_TABS.map(({ id, icon: Icon, label }) => (
        <Tooltip key={id} label={label}>
          <motion.button className={`e-ric${id === "ai" ? " ai" : ""}${tab === id ? " on" : ""}`}
            title={label} aria-label={label} aria-current={tab === id || undefined} onClick={() => onTab(id)}
            whileTap={PRESS_TAP} transition={PRESS_SPRING}>
            {tab === id && (
              <motion.span className="e-ric-hl" layoutId="rail-active"
                transition={{ type: "tween", duration: 0.18, ease: [0.4, 0, 0.2, 1] }} />
            )}
            <Icon size={20} />
          </motion.button>
        </Tooltip>
      ))}
    </div>
  );
}
