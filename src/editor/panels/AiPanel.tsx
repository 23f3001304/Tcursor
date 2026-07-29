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

/** Last-resort placeholder shown only when Ollama is unreachable / has no models installed (so the
 *  picker is never empty). When models ARE installed, the panel selects a real local one. */
const FALLBACK_MODEL = "llama3.2";

/** The AI Director rail panel: a real Engine picker (installed Ollama models), the auto-edit
 *  run button, and what it does. */
export function AiPanel({
  running, error, log, onRun, model, onChangeModel,
}: {
  running: boolean; error: string | null; log: string[]; onRun: () => void; model: string; onChangeModel: (v: string) => void;
}) {
  const [models, setModels] = useState<string[]>([]);
  useEffect(() => { listOllamaModels().then(setModels).catch(() => {}); }, []);
  // Once the local model list loads, default to a real installed model whenever the saved one is
  // missing/empty - so Auto-edit never sends a model that isn't pulled (Ollama 404s on those).
  useEffect(() => {
    if (models.length && !models.includes(model)) onChangeModel(models[0]);
  }, [models, model, onChangeModel]);

  // Show an installed model as selected even before the default above persists to the doc.
  const current = models.includes(model) ? model : (models[0] ?? model ?? FALLBACK_MODEL);
  const options = (models.length ? models : [current]).map((m) => ({ value: m, label: m }));

  return (
    <div className="e-panel">
      <h2>AI Director</h2>
      <p className="e-lede">Edits from your recording events - clicks, keystrokes, cursor. Runs on your machine via Ollama.</p>
      <div className="e-field">
        <span className="e-fl">Engine</span>
        <Picker value={current} options={options} onChange={onChangeModel} />
      </div>
      <button className="e-run" onClick={onRun} disabled={running} style={{ marginTop: 16 }}>
        {running ? <><Spin size={16} />Directing…</> : <><IconSparkles size={16} />Auto-edit</>}
      </button>
      {error && <p className="e-ai-err" role="alert">{error}</p>}
      {log.length > 0 ? (
        // Agentic reveal: each edit the director applies streams in here as a narration line.
        <div className="e-ai-log">
          {log.map((line, i) => (
            <motion.div key={i} className={`e-ai-log-line${line.startsWith("✓") ? " done" : ""}`}
              initial={{ opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }}
              transition={{ type: "tween", duration: 0.24, ease: [0.4, 0, 0.2, 1] }}>
              {line}
            </motion.div>
          ))}
        </div>
      ) : (
        <ul className="e-sum">
          {SUMMARY.map(([Icon, txt], i) => (
            <motion.li key={i} initial={{ opacity: 0, y: 5 }} animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.1 + i * 0.05 }}>
              <span className="ic"><Icon size={15} /></span>{txt}
            </motion.li>
          ))}
        </ul>
      )}
    </div>
  );
}
