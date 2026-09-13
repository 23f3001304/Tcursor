import { motion } from "motion/react";
import { IconX, IconRotate2 } from "@tabler/icons-react";
// Every panel and inspector renders this header, so it is where the panel stylesheet enters the
// bundle - the same way `Controls.tsx` carries `controls.css`.
import "./panels.css";

// design/premium-pass D6: the app-wide press spring (scale .96, stiffness 500, damping 30) -
// these ghost icon buttons had no press feedback at all before this.
const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

// Shared panel/inspector header: title + a hairline, with matched ghost icon buttons
// (reset + close) so every panel reads the same. Reset is optional.
export function PanelHeader({ title, lede, thumb, onReset, onClose, closeTitle = "Close" }: {
  title: string;
  lede?: string;
  /** An optional small visual shown beside the title (T34 L4: `LayoutInspector`'s 48x28
   *  arrangement schematic, `LayoutThumb.tsx`) - generic `ReactNode` so this component stays
   *  agnostic of what it actually is. Wrapped into the SAME flex group as `<h2>` (not a third
   *  `.e-phead-top` child) so `justify-content: space-between` still only ever balances two
   *  things: that group against `.e-hicons`. Omitted (the other 12+ call sites), it's a no-op -
   *  a lone `<h2>` in a `.e-phead-title` reads identically to the old bare `<h2>`. */
  thumb?: React.ReactNode;
  onReset?: () => void;
  onClose: () => void;
  closeTitle?: string;
}) {
  return (
    <div className="e-phead">
      <div className="e-phead-top">
        <div className="e-phead-title">{thumb}<h2>{title}</h2></div>
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
      {/* One line at hint size (panels.css). `title` keeps the whole sentence reachable when a
          longer lede - an inspector's measured one, say - runs past the panel's 320px. */}
      {lede && <p className="e-lede" title={lede}>{lede}</p>}
    </div>
  );
}
