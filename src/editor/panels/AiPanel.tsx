import { useCallback, useEffect, useState, type ComponentType } from "react";
import { motion } from "motion/react";
import { IconSparkles, IconZoomIn, IconVideo, IconCut } from "@tabler/icons-react";
import { PanelHeader } from "./PanelHeader";
import { Spin } from "../controls/Spin";
import { Picker } from "../controls/Controls";
import { Shimmer } from "../timeline/Shimmer";
import { listOllamaModels } from "../../lib/ipc";
import { friendlyAiError } from "../director/friendlyAiError";

const SUMMARY: [ComponentType<{ size?: number }>, string][] = [
  [IconZoomIn, "Places zooms on your clicks"],
  [IconVideo, "Punches the camera in"],
  [IconCut, "Trims idle gaps"],
];

/** The AI Director rail panel: a real Engine picker (installed Ollama models), the auto-edit
 *  run button, and what it does. */
export function AiPanel({
  running, error, log, onRun, model, onChangeModel, onAutoModel, progress, onClose,
}: {
  running: boolean; error: string | null; log: string[]; onRun: () => void; model: string; onChangeModel: (v: string) => void;
  onAutoModel: (v: string) => void;
  progress: { step: number; total: number } | null; // live "k of N" while the director is running
  // AiPanel IS the "ai" tab (the router's home/fallback) - there's nowhere else for it to close
  // TO. PanelHeader's close icon is not optional, so this is wired the same way every other
  // panel's is (back to the "ai" tab) for a consistent header across all panels, even though on
  // this one it's a no-op (already there).
  onClose: () => void;
}) {
  // `null` = fetch in flight; `[]` (after resolving, or on rejection) = genuinely no local model
  // available - both render an honest state instead of ever showing the hardcoded fallback name
  // as if it were an installed, selectable model.
  const [models, setModels] = useState<string[] | null>(null);
  const loadModels = useCallback(() => {
    setModels(null);
    listOllamaModels().then(setModels).catch(() => setModels([]));
  }, []);
  useEffect(loadModels, [loadModels]);

  // Once the local model list loads, default to a real installed model whenever the saved one is
  // missing/empty - so Auto-edit never sends a model that isn't pulled (Ollama 404s on those).
  // Uses `onAutoModel` (a QUIET write - no undo-history push) rather than `onChangeModel`: this
  // fires whenever the model list resolves, not from the user touching the picker, so it must not
  // create a phantom undo step or an unasked-for disk write just from opening the panel.
  useEffect(() => {
    if (models && models.length && !models.includes(model)) onAutoModel(models[0]);
  }, [models, model, onAutoModel]);

  const current = models?.includes(model) ? model : (models?.[0] ?? model);
  const options = (models ?? []).map((m) => ({ value: m, label: m }));
  const errInfo = error ? friendlyAiError(error) : null;

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="AI Director" lede="Edits from your recording events - clicks, keystrokes, cursor. Runs on your machine via Ollama." onClose={onClose} />
      <div className="e-field">
        <span className="e-fl">Engine</span>
        {models === null ? (
          <Shimmer className="e-picker-shell" />
        ) : models.length === 0 ? (
          <p className="e-errline" role="alert">
            Ollama isn't running or has no models — start Ollama, then
            <button type="button" onClick={loadModels}>Retry</button>
          </p>
        ) : (
          <Picker value={current} options={options} onChange={onChangeModel} ariaLabel="Engine" />
        )}
      </div>
      <button className="e-run" onClick={onRun} disabled={running || !models?.length} style={{ marginTop: 16 }} data-director-anchor="wand">
        {running
          ? <><Spin size={16} />Directing{progress ? `… ${progress.step} of ${progress.total}` : "…"}</>
          : <><IconSparkles size={16} />Auto-edit</>}
      </button>
      {running && progress && (
        <div className="e-ai-progress">
          <motion.div className="e-ai-progress-fill" initial={false}
            animate={{ width: `${(progress.step / progress.total) * 100}%` }}
            transition={{ type: "tween", duration: 0.2, ease: [0.4, 0, 0.2, 1] }} />
        </div>
      )}
      {errInfo && (
        <p className="e-ai-err" role="alert">{errInfo.title}{errInfo.hint && <span className="hint">{errInfo.hint}</span>}</p>
      )}
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
