import { useCallback, useEffect, useState, type ComponentType } from "react";
import { AnimatePresence, motion } from "motion/react";
import { IconSparkles, IconZoomIn, IconVideo, IconCut } from "@tabler/icons-react";
import { PanelHeader } from "./PanelHeader";
import { Spin } from "../controls/surfaces/Spin";
import { Picker } from "../controls/Controls";
import { Shimmer } from "../timeline/lanes/Shimmer";
import { listOllamaModels, type OllamaModel } from "../../shared/ipc";
import { friendlyAiError } from "../director/friendlyAiError";
import { engineDisplayName } from "../director/engineName";
import { ReviewSheet } from "../director/review/ReviewSheet";
import type { AiRun } from "../../shared/aiRun";

const HINT_MOTION = {
  initial: { opacity: 0, y: -4 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -4 },
  transition: { duration: 0.14 },
};

const SUMMARY: [ComponentType<{ size?: number }>, string][] = [
  [IconZoomIn, "Zooms on clicks"],
  [IconCut, "Removes idle time"],
  [IconVideo, "Emphasizes important actions"],
];

export function AiPanel({
  running,
  exporting,
  error,
  onRun,
  model,
  onChangeModel,
  onAutoModel,
  progress,
  onClose,
  run,
  skipped,
  applying,
  previewId,
  onToggleItem,
  onPreviewItem,
  onApply,
  onDiscard,
}: {
  running: boolean;
  exporting: boolean;
  error: string | null;
  onRun: () => void;
  model: string;
  onChangeModel: (v: string) => void;
  onAutoModel: (v: string) => void;
  progress: { step: number; total: number } | null;
  run: AiRun | null;
  skipped: ReadonlySet<string>;
  applying: boolean;
  previewId: string | null;
  onToggleItem: (id: string) => void;
  onPreviewItem: (id: string) => void;
  onApply: () => void;
  onDiscard: () => void;
  onClose: () => void;
}) {
  const [models, setModels] = useState<OllamaModel[] | null>(null);
  const loadModels = useCallback(() => {
    setModels(null);
    listOllamaModels()
      .then(setModels)
      .catch(() => setModels([]));
  }, []);
  useEffect(loadModels, [loadModels]);

  useEffect(() => {
    if (models && models.length && !models.some((m) => m.name === model)) onAutoModel(models[0].name);
  }, [models, model, onAutoModel]);

  const current = models?.some((m) => m.name === model) ? model : (models?.[0]?.name ?? model);
  const options = (models ?? []).map((m) => ({
    value: m.name,
    label: engineDisplayName(m.name),
    title: m.name,
    badge: m.vision ? "Vision" : undefined,
  }));
  const errInfo = error ? friendlyAiError(error) : null;
  const noModelsTitle =
    models !== null && models.length === 0
      ? "No local Ollama models found - install one (`ollama pull <model>`), then Retry below."
      : undefined;

  return (
    <div className="e-panel e-insp">
      <PanelHeader
        title="AI Director"
        lede="Auto-editing by a model running on this machine."
        onClose={onClose}
      />

      <div className="e-grp">
        <button
          className="e-run"
          onClick={onRun}
          disabled={running || exporting || !models?.length}
          title={noModelsTitle}
          data-director-anchor="wand"
        >
          {running ? (
            <>
              <Spin size={16} />
              {progress ? `Replaying... ${progress.step} of ${progress.total}` : "Thinking..."}
            </>
          ) : (
            <>
              <IconSparkles size={16} />
              Auto-edit
            </>
          )}
        </button>
        <AnimatePresence>
          {running && progress && (
            <motion.div className="e-ai-progress" {...HINT_MOTION}>
              <motion.div
                className="e-ai-progress-fill"
                initial={false}
                animate={{ width: `${(progress.step / progress.total) * 100}%` }}
                transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
              />
            </motion.div>
          )}
        </AnimatePresence>
        <AnimatePresence>
          {errInfo && (
            <motion.p className="e-ai-err" role="alert" title={error ?? undefined} {...HINT_MOTION}>
              {errInfo.title}
              <button type="button" onClick={onRun} disabled={running || exporting}>
                Retry
              </button>
              {errInfo.hint && <span className="hint">{errInfo.hint}</span>}
            </motion.p>
          )}
        </AnimatePresence>
      </div>

      <AnimatePresence mode="wait" initial={false}>
        {run && (
          <motion.div key="sheet" {...HINT_MOTION}>
            <ReviewSheet
              run={run}
              skipped={skipped}
              applying={applying}
              previewId={previewId}
              onToggle={onToggleItem}
              onPreview={onPreviewItem}
              onApply={onApply}
              onDiscard={onDiscard}
            />
          </motion.div>
        )}
      </AnimatePresence>
      {!run && (
        <div className="e-grp">
          <p className="e-ai-what">Automatically finds important moments and creates camera movement.</p>
          <ul className="e-sum">
            {SUMMARY.map(([Icon, txt], i) => (
              <motion.li
                key={i}
                initial={{ opacity: 0, y: 5 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1], delay: 0.1 + i * 0.05 }}
              >
                <Icon size={14} />
                {txt}
              </motion.li>
            ))}
          </ul>
        </div>
      )}

      <div className="e-ai-engine">
        <span className="e-ai-engine-l">Engine</span>
        {models === null ? (
          <Shimmer className="e-picker-shell" />
        ) : models.length === 0 ? (
          <div className="e-picker-shell e-picker-empty" role="status" title={noModelsTitle}>
            <span>None found</span>
            <button type="button" onClick={loadModels} title="Look for installed Ollama models again">
              Retry
            </button>
          </div>
        ) : (
          <Picker value={current} options={options} onChange={onChangeModel} ariaLabel="Engine" />
        )}
      </div>
    </div>
  );
}
