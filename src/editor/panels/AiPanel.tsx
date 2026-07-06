import type { ComponentType } from "react";
import { motion } from "motion/react";
import { IconChevronDown, IconSparkles, IconZoomIn, IconVideo, IconCut } from "@tabler/icons-react";
import { Spin } from "../controls/Spin";

const SUMMARY: [ComponentType<{ size?: number }>, string][] = [
  [IconZoomIn, "Places zooms on your clicks"],
  [IconVideo, "Punches the camera in"],
  [IconCut, "Trims idle gaps"],
];

/** The AI Director rail panel: engine/style pickers, the auto-edit run button, what it does. */
export function AiPanel({ running, onRun }: { running: boolean; onRun: () => void }) {
  return (
    <div className="e-panel">
      <h2>AI Director</h2>
      <p className="e-lede">Edits from your recording events - clicks, keystrokes, cursor. Runs on your machine.</p>
      <button className="e-sel"><span className="l">Engine</span>
        <span className="v"><span className="lc" />Ollama<IconChevronDown size={15} /></span></button>
      <button className="e-sel"><span className="l">Style</span>
        <span className="v">Demo<IconChevronDown size={15} /></span></button>
      <button className="e-run" onClick={onRun} disabled={running}>
        {running ? <><Spin size={16} />Editing...</> : <><IconSparkles size={16} />Auto-edit</>}
      </button>
      <ul className="e-sum">
        {SUMMARY.map(([Icon, txt], i) => (
          <motion.li key={i} initial={{ opacity: 0, y: 5 }} animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 + i * 0.05 }}>
            <span className="ic"><Icon size={15} /></span>{txt}
          </motion.li>
        ))}
      </ul>
    </div>
  );
}
