import { motion } from "motion/react";
import { IconX, IconRotate2 } from "@tabler/icons-react";

import "./panels.css";

const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

export function PanelHeader({
  title,
  lede,
  thumb,
  onReset,
  onClose,
  closeTitle = "Close",
}: {
  title: string;
  lede?: string;
  thumb?: React.ReactNode;
  onReset?: () => void;
  onClose: () => void;
  closeTitle?: string;
}) {
  return (
    <div className="e-phead">
      <div className="e-phead-top">
        <div className="e-phead-title">
          {thumb}
          <h2>{title}</h2>
        </div>
        <div className="e-hicons">
          {onReset && (
            <motion.button
              type="button"
              className="e-hicon"
              title="Reset to defaults"
              aria-label="Reset to defaults"
              onClick={onReset}
              whileTap={PRESS_TAP}
              transition={PRESS_SPRING}
            >
              <IconRotate2 size={15} />
            </motion.button>
          )}
          <motion.button
            type="button"
            className="e-hicon"
            title={closeTitle}
            aria-label={closeTitle}
            onClick={onClose}
            whileTap={PRESS_TAP}
            transition={PRESS_SPRING}
          >
            <IconX size={15} />
          </motion.button>
        </div>
      </div>
      {lede && (
        <p className="e-lede" title={lede}>
          {lede}
        </p>
      )}
    </div>
  );
}
