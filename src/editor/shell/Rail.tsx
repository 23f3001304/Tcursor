import { motion } from "motion/react";
import { Tooltip } from "../controls/surfaces/Tooltip";
import { PANEL_TABS, type Tab } from "./PanelTabs";

const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

export function Rail({ tab, onTab }: { tab: Tab | null; onTab: (t: Tab) => void }) {
  return (
    <div className="e-rail">
      {PANEL_TABS.map(({ id, icon: Icon, label }) => (
        <Tooltip key={id} label={label}>
          <motion.button
            className={`e-ric${id === "ai" ? " ai" : ""}${tab === id ? " on" : ""}`}
            title={label}
            aria-label={label}
            aria-current={tab === id || undefined}
            onClick={() => onTab(id)}
            whileTap={PRESS_TAP}
            transition={PRESS_SPRING}
          >
            {tab === id && (
              <motion.span
                className="e-ric-hl"
                layoutId="rail-active"
                transition={{ type: "tween", duration: 0.18, ease: [0.4, 0, 0.2, 1] }}
              />
            )}
            <Icon size={20} />
          </motion.button>
        </Tooltip>
      ))}
    </div>
  );
}
