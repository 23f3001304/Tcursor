import { useEffect, useRef, useState } from "react";
import { motion } from "motion/react";
import { IconCheck, IconAlertTriangle, IconFolderOpen } from "@tabler/icons-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Spin } from "../controls/Spin";
import { fmt } from "../timeline/time";
import { estimateEtaMs } from "./exportEta";

/** The export progress/outcome view inside `ExportDialog`: a live bar with %/ETA while exporting,
 *  then a done checkmark or an error banner. Owns the "when did this export start" clock (reset
 *  whenever `exporting` flips false -> true) so the ETA has an elapsed baseline without the
 *  caller needing to track wall-clock time itself. */
export function ExportProgress({ exporting, pct, done, error, exportPath, onReset, onClose }: {
  exporting: boolean; pct: number; done: boolean; error: string | null; exportPath: string;
  onReset: () => void; onClose: () => void;
}) {
  const startRef = useRef<number | null>(null);
  const [etaMs, setEtaMs] = useState<number | null>(null);

  useEffect(() => {
    if (exporting && startRef.current === null) startRef.current = Date.now();
    if (!exporting) startRef.current = null;
  }, [exporting]);

  useEffect(() => {
    if (!exporting || startRef.current === null) { setEtaMs(null); return; }
    setEtaMs(estimateEtaMs(Date.now() - startRef.current, pct));
  }, [exporting, pct]);

  if (error) {
    return (
      <div className="e-export-outcome">
        <IconAlertTriangle size={28} className="e-export-icon error" />
        <p className="e-export-outcome-title">Export failed</p>
        <p className="e-export-outcome-body">{error}</p>
        <div className="e-modal-actions">
          <button type="button" className="e-modal-btn" onClick={onClose}>Close</button>
          <button type="button" className="e-modal-btn primary" onClick={onReset}>Try again</button>
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
          <button type="button" className="e-modal-btn primary" onClick={onClose}>Done</button>
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
