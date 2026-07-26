import { useEffect, useState, type ComponentType } from "react";
import { motion } from "motion/react";
import { IconSparkles, IconZoomIn, IconVideo, IconCut } from "@tabler/icons-react";
import { Spin } from "../controls/Spin";
import { Picker } from "../controls/Controls";
import { listOllamaModels } from "../../lib/ipc";

const SUMMARY: [ComponentType<{ size?: number }>, string][] = [
  [IconZoomIn, "Places zooms on your clicks"],
  [IconVideo, "Punches the camera in"],
  [IconCut, "Trims idle gaps"],
];

/** Ollama's own default model name (mirrors the Rust fallback in `ai::commands::ai_autoedit`),
 *  shown/selected whenever no model is chosen yet or Ollama's model list hasn't loaded. */
const FALLBACK_MODEL = "llama3.2";

/** The AI Director rail panel: a real Engine picker (installed Ollama models), the auto-edit
 *  run button, and what it does. */
export function AiPanel({
  running, onRun, model, onChangeModel,
}: {
  running: boolean; onRun: () => void; model: string; onChangeModel: (v: string) => void;
}) {
  const [models, setModels] = useState<string[]>([]);
  useEffect(() => { listOllamaModels().then(setModels).catch(() => {}); }, []);

  const current = model || FALLBACK_MODEL;
  const options = (models.length ? models : [current]).map((m) => ({ value: m, label: m }));

  return (
    <div className="e-panel">
      <h2>AI Director</h2>
      <p className="e-lede">Edits from your recording events - clicks, keystrokes, cursor. Runs on your machine via Ollama.</p>
      <div className="e-field">
        <span className="e-fl">Engine</span>
        <Picker value={current} options={options} onChange={onChangeModel} />
      </div>
      <button className="e-run" onClick={onRun} disabled={running} style={{ marginTop: 12 }}>
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
