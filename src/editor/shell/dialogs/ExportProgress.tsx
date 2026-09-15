import { useEffect, useState } from "react";
import { motion } from "motion/react";
import { IconAlertTriangle, IconFolderOpen } from "@tabler/icons-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { SweepWave } from "../../../shared/wave/ui/SweepWave";
import { fmt } from "../../timeline/model/time";
import { estimateEtaMs } from "./exportEta";

const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

export function ExportProgress({
  exporting,
  pct,
  done,
  error,
  exportPath,
  startedAt,
  onReset,
  onClose,
}: {
  exporting: boolean;
  pct: number;
  done: boolean;
  error: string | null;
  exportPath: string;
  startedAt: number | null;
  onReset: () => void;
  onClose: () => void;
}) {
  const [etaMs, setEtaMs] = useState<number | null>(null);

  useEffect(() => {
    if (!exporting || startedAt === null) {
      setEtaMs(null);
      return;
    }
    setEtaMs(estimateEtaMs(Date.now() - startedAt, pct));
  }, [exporting, pct, startedAt]);

  if (error) {
    return (
      <div className="e-export-outcome">
        <IconAlertTriangle size={28} className="e-export-icon error" />
        <p className="e-export-outcome-title">Export failed</p>
        <p className="e-export-outcome-body">{error}</p>
        <div className="e-modal-actions">
          <button type="button" className="e-modal-btn" onClick={onClose}>
            Close
          </button>
          <motion.button
            type="button"
            className="e-modal-btn primary"
            onClick={onReset}
            whileTap={PRESS_TAP}
            transition={PRESS_SPRING}
          >
            Try again
          </motion.button>
        </div>
      </div>
    );
  }

  if (done) {
    return (
      <div className="e-export-outcome">
        <SweepWave w={200} h={30} pct={100} done />
        <p className="e-export-outcome-title">Export complete</p>
        <div className="e-modal-actions">
          <button type="button" className="e-modal-btn" onClick={onReset}>
            Export again
          </button>
          {exportPath && (
            <button
              type="button"
              className="e-modal-btn"
              onClick={() => void revealItemInDir(exportPath).catch(() => {})}
            >
              <IconFolderOpen size={14} />
              Show in folder
            </button>
          )}
          <motion.button
            type="button"
            className="e-modal-btn primary"
            onClick={onClose}
            whileTap={PRESS_TAP}
            transition={PRESS_SPRING}
          >
            Done
          </motion.button>
        </div>
      </div>
    );
  }

  return (
    <div className="e-export-progress">
      <div className="e-export-progress-row">
        <span>Exporting...</span>
        <span className="e-export-pct">{pct}%</span>
      </div>
      <SweepWave w={352} h={34} pct={pct} />
      <p className="e-export-eta">
        {etaMs === null ? "Estimating time remaining..." : `About ${fmt(etaMs)} remaining`}
      </p>
    </div>
  );
}
