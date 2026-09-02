import { motion } from "motion/react";
import { IconX, IconRotate2 } from "@tabler/icons-react";

// design/premium-pass D6: the app-wide press spring (scale .96, stiffness 500, damping 30) -
// these ghost icon buttons had no press feedback at all before this.
const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

// Shared panel/inspector header: title + a hairline, with matched ghost icon buttons
// (reset + close) so every panel reads the same. Reset is optional.
export function PanelHeader({ title, lede, onReset, onClose, closeTitle = "Close" }: {
  title: string;
  lede?: string;
  onReset?: () => void;
  onClose: () => void;
  closeTitle?: string;
}) {
  return (
    <div className="e-phead">
      <div className="e-phead-top">
        <h2>{title}</h2>
        <div className="e-hicons">
          {onReset && (
            <motion.button type="button" className="e-hicon" title="Reset to defaults" aria-label="Reset to defaults" onClick={onReset}
              whileTap={PRESS_TAP} transition={PRESS_SPRING}>
              <IconRotate2 size={15} />
            </motion.button>
          )}
          <motion.button type="button" className="e-hicon" title={closeTitle} aria-label={closeTitle} onClick={onClose}
            whileTap={PRESS_TAP} transition={PRESS_SPRING}>
            <IconX size={15} />
          </motion.button>
        </div>
      </div>
      {lede && <p className="e-lede">{lede}</p>}
    </div>
  );
}
