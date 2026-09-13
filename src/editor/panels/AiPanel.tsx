import { useCallback, useEffect, useState, type ComponentType } from "react";
import { AnimatePresence, motion } from "motion/react";
import { IconSparkles, IconZoomIn, IconVideo, IconCut } from "@tabler/icons-react";
import { PanelHeader } from "./PanelHeader";
import { Spin } from "../controls/Spin";
import { Picker } from "../controls/Controls";
import { Shimmer } from "../timeline/Shimmer";
import { listOllamaModels } from "../../lib/ipc";
import { friendlyAiError } from "../director/friendlyAiError";
import { engineDisplayName } from "../director/engineName";

// design/premium-pass D6: the progress bar and error row below both pop as the director runs -
// a cheap opacity/y-4 tween, consistent with the existing dialog enters.
const HINT_MOTION = { initial: { opacity: 0, y: -4 }, animate: { opacity: 1, y: 0 }, exit: { opacity: 0, y: -4 }, transition: { duration: 0.14 } };

const SUMMARY: [ComponentType<{ size?: number }>, string][] = [
  [IconZoomIn, "Places zooms on your clicks"],
  [IconVideo, "Punches the camera in"],
  [IconCut, "Trims idle gaps"],
];

/** The AI Director rail panel: a real Engine picker (installed Ollama models), the auto-edit
 *  run button, and what it does. */
export function AiPanel({
  running, exporting, error, log, onRun, model, onChangeModel, onAutoModel, progress, onClose,
}: {
  running: boolean;
  exporting: boolean; // locks the run button too - see Editor.tsx's `onRun`/Transport's wand (L3)
  error: string | null; log: string[]; onRun: () => void; model: string; onChangeModel: (v: string) => void;
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
  // Raw Ollama ids can be very long, especially HF GGUF proxies (ux audit #16: wraps over two
  // lines in this dropdown) - `label` is the short derived name, `title` keeps the full id on
  // hover so it's never actually hidden, just not the headline.
  const options = (models ?? []).map((m) => ({ value: m, label: engineDisplayName(m), title: m }));
  const errInfo = error ? friendlyAiError(error) : null;
  // `models !== null` guards this to ONLY the resolved-and-empty state (not "still loading",
  // which is already communicated by the Shimmer) - gate finding: the disabled run button gave
  // no reason why, so a user with no models pulled had nothing to go on but a dead button.
  const noModelsTitle = models !== null && models.length === 0
    ? "No local Ollama models found - install one (`ollama pull <model>`), then Retry above."
    : undefined;

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="AI Director" lede="Local edits from your clicks and keystrokes, via Ollama." onClose={onClose} />

      {/* The choice this panel is about, first: which local model does the editing. */}
      <div className="e-grp">
        <div className="e-field">
          <span className="e-fl">Engine</span>
          {models === null ? (
            <Shimmer className="e-picker-shell" />
          ) : models.length === 0 ? (
            <div className="e-picker-shell e-picker-empty" role="status" title={noModelsTitle}>
              <span>No local models found</span>
              <button type="button" onClick={loadModels} title="Look for installed Ollama models again">Retry</button>
            </div>
          ) : (
            <Picker value={current} options={options} onChange={onChangeModel} ariaLabel="Engine" />
          )}
        </div>
        <button className="e-run" onClick={onRun} disabled={running || exporting || !models?.length}
          title={noModelsTitle} data-director-anchor="wand">
          {running
            ? <><Spin size={16} />Directing{progress ? `... ${progress.step} of ${progress.total}` : "..."}</>
            : <><IconSparkles size={16} />Auto-edit</>}
        </button>
        <AnimatePresence>
          {running && progress && (
            <motion.div className="e-ai-progress" {...HINT_MOTION}>
              <motion.div className="e-ai-progress-fill" initial={false}
                animate={{ width: `${(progress.step / progress.total) * 100}%` }}
                transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }} />
            </motion.div>
          )}
        </AnimatePresence>
        <AnimatePresence>
          {errInfo && (
            <motion.p className="e-ai-err" role="alert" {...HINT_MOTION}>
              {errInfo.title}{errInfo.hint && <span className="hint">{errInfo.hint}</span>}
            </motion.p>
          )}
        </AnimatePresence>
      </div>

      {/* Then what it did, or - before a run - what it will do. */}
      <div className="e-grp">
        <span className="e-sechead">{log.length > 0 ? "Run" : "What it does"}</span>
        {log.length > 0 ? (
          // Agentic reveal: each edit the director applies streams in here as a narration line.
          <div className="e-ai-log">
            {log.map((line, i) => (
              <motion.div key={i} className={`e-ai-log-line${line.startsWith("✓") ? " done" : ""}`}
                initial={{ opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }}
                transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}>
                {line}
              </motion.div>
            ))}
          </div>
        ) : (
          <ul className="e-sum">
            {SUMMARY.map(([Icon, txt], i) => (
              <motion.li key={i} initial={{ opacity: 0, y: 5 }} animate={{ opacity: 1, y: 0 }}
                transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1], delay: 0.1 + i * 0.05 }}>
                <span className="ic"><Icon size={15} /></span>{txt}
              </motion.li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
