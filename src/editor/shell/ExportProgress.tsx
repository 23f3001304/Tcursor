import { useEffect, useState } from "react";
import { motion } from "motion/react";
import { IconCheck, IconAlertTriangle, IconFolderOpen } from "@tabler/icons-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Spin } from "../controls/Spin";
import { fmt } from "../timeline/time";
import { estimateEtaMs } from "./exportEta";

// design/premium-pass D6: the app-wide press spring, for the primary CTA only.
const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

/** The export progress/outcome view inside `ExportDialog`: a live bar with %/ETA while exporting,
 *  then a done checkmark or an error banner. `startedAt` (the export's own start time) is owned
 *  by the caller (Editor.tsx's useExportState), not tracked locally here - this used to reset its
 *  own `useRef` clock on mount, which meant closing and reopening the dialog mid-export (a
 *  supported flow) unmounted/remounted this component and silently reset the ETA baseline back to
 *  "just started" (D Low). */
export function ExportProgress({ exporting, pct, done, error, exportPath, startedAt, onReset, onClose }: {
  exporting: boolean; pct: number; done: boolean; error: string | null; exportPath: string;
  startedAt: number | null;
  onReset: () => void; onClose: () => void;
}) {
  const [etaMs, setEtaMs] = useState<number | null>(null);

  useEffect(() => {
    if (!exporting || startedAt === null) { setEtaMs(null); return; }
    setEtaMs(estimateEtaMs(Date.now() - startedAt, pct));
  }, [exporting, pct, startedAt]);

  if (error) {
    return (
      <div className="e-export-outcome">
        <IconAlertTriangle size={28} className="e-export-icon error" />
        <p className="e-export-outcome-title">Export failed</p>
        <p className="e-export-outcome-body">{error}</p>
        <div className="e-modal-actions">
          <button type="button" className="e-modal-btn" onClick={onClose}>Close</button>
          <motion.button type="button" className="e-modal-btn primary" onClick={onReset}
            whileTap={PRESS_TAP} transition={PRESS_SPRING}>Try again</motion.button>
        </div>
      </div>
    );
  }

  if (done) {
    return (
      <div className="e-export-outcome">
        <IconCheck size={28} className="e-export-icon done" />
        <p className="e-export-outcome-title">Export complete</p>
        <div className="e-modal-actions">
          <button type="button" className="e-modal-btn" onClick={onReset}>Export again</button>
          {exportPath && (
            <button type="button" className="e-modal-btn" onClick={() => void revealItemInDir(exportPath).catch(() => {})}>
              <IconFolderOpen size={14} />Show in folder
            </button>
          )}
          <motion.button type="button" className="e-modal-btn primary" onClick={onClose}
            whileTap={PRESS_TAP} transition={PRESS_SPRING}>Done</motion.button>
        </div>
      </div>
    );
  }

  return (
    <div className="e-export-progress">
      <div className="e-export-progress-row">
        <Spin size={15} />
        <span>Exporting...</span>
        <span className="e-export-pct">{pct}%</span>
      </div>
      <div className="e-export-bar-track">
        <motion.div className="e-export-bar-fill" animate={{ width: `${pct}%` }}
          transition={{ type: "spring", stiffness: 120, damping: 24 }} />
      </div>
      <p className="e-export-eta">
        {etaMs === null ? "Estimating time remaining..." : `About ${fmt(etaMs)} remaining`}
      </p>
    </div>
  );
}
