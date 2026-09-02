import type { ComponentType } from "react";
import { motion } from "motion/react";
import { IconSparkles, IconPhoto, IconPointer, IconCamera, IconLetterCase, IconVolume, IconWand } from "@tabler/icons-react";

export type Tab = "ai" | "background" | "cursor" | "camera" | "captions" | "audio" | "effects";

/** The one press spring the whole app's quiet chrome uses (design/premium-pass D6) - scale .96,
 *  no hover bump (unlike Transport's TAP_SPRING), since a rail tab's active-pill glide is already
 *  the primary hover signal. */
const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

const TABS: { id: Tab; icon: ComponentType<{ size?: number }>; label: string }[] = [
  { id: "ai", icon: IconSparkles, label: "AI Director" },
  { id: "background", icon: IconPhoto, label: "Background" },
  { id: "cursor", icon: IconPointer, label: "Cursor" },
  { id: "camera", icon: IconCamera, label: "Camera" },
  { id: "captions", icon: IconLetterCase, label: "Captions" },
  { id: "audio", icon: IconVolume, label: "Audio" },
  { id: "effects", icon: IconWand, label: "Effects" },
];

/** Left icon rail: one button per editor tab. The active pill is a single shared motion.span
 *  (layoutId "rail-active") that glides between tools; the icon color still snaps (.on),
 *  and "ai" keeps the violet tint. */
export function Rail({ tab, onTab }: { tab: Tab; onTab: (t: Tab) => void }) {
  return (
    <div className="e-rail">
      {TABS.map(({ id, icon: Icon, label }) => (
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
