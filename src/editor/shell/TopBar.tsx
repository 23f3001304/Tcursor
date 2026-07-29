import { useState } from "react";
import { motion } from "motion/react";
import { getCurrentWindow, LogicalSize, LogicalPosition } from "@tauri-apps/api/window";
import { IconPointer, IconArrowLeft, IconArrowBackUp, IconArrowForwardUp, IconDownload, IconMinus, IconMaximize, IconX } from "@tabler/icons-react";
import { Spin } from "../controls/Spin";

/** Editor top bar: drag region, back-to-recorder, brand, undo/redo, GitHub, Export,
 *  and window minimize/maximize/close. Non-button children are pointer-events:none (in
 *  CSS) so the empty bar area drags the window while the buttons stay clickable. Maximize
 *  toggles via setSize (native maximize no-ops on this transparent window); the back
 *  arrow returns to the HUD (onClose); Close (X) quits the app (closes the sole window).
 *  The Export button opens `ExportDialog` (`onOpenExport`) rather than exporting immediately;
 *  `exporting`/`pct` still drive this bar's own mini progress fill once a dialog-started export
 *  is actually running, even after the dialog itself is closed. */
export function TopBar({ proj, exporting, pct, onOpenExport, onClose, onUndo, onRedo, canUndo, canRedo }: {
  proj: string; exporting: boolean; pct: number; onOpenExport: () => void; onClose: () => void;
  onUndo: () => void; onRedo: () => void; canUndo: boolean; canRedo: boolean;
}) {
  const win = getCurrentWindow();
  const [maxed, setMaxed] = useState(false);
  const toggleMax = async () => {
    if (maxed) {
      const w = Math.min(1440, window.screen.availWidth - 120), h = Math.min(900, window.screen.availHeight - 120);
      await win.setSize(new LogicalSize(w, h));
      await win.center();
    } else {
      await win.setSize(new LogicalSize(window.screen.width, window.screen.height));
      await win.setPosition(new LogicalPosition(0, 0));
    }
    setMaxed((m) => !m);
  };
  return (
    <div className="e-bar" data-tauri-drag-region>
      <button className="e-gst" title="Back to recorder" onClick={onClose}><IconArrowLeft size={18} /></button>
      <div className="e-brand">
        <span className="e-mark"><IconPointer size={12} /></span>TCursor
        <span className="e-proj">{proj}</span>
      </div>
      <div className="e-sp" />
      <button className="e-gst" title="Undo (Ctrl+Z)" onClick={onUndo} disabled={!canUndo}><IconArrowBackUp size={18} /></button>
      <button className="e-gst" title="Redo (Ctrl+Shift+Z)" onClick={onRedo} disabled={!canRedo}><IconArrowForwardUp size={18} /></button>
      <button className="e-export" onClick={onOpenExport} disabled={exporting}>
        {exporting ? <><Spin size={15} />{pct}%</> : <><IconDownload size={15} />Export</>}
        {exporting && (
          <motion.div className="e-export-bar" animate={{ width: `${pct}%` }}
            transition={{ type: "spring", stiffness: 120, damping: 24 }} />
        )}
      </button>
      <span className="e-bardiv" />
      <button className="e-gst" title="Minimize" onClick={() => void win.minimize()}><IconMinus size={18} /></button>
      <button className="e-gst" title={maxed ? "Restore" : "Maximize"} onClick={() => void toggleMax()}><IconMaximize size={16} /></button>
      <button className="e-gst close" title="Close" onClick={() => void win.close()}><IconX size={18} /></button>
    </div>
  );
}
