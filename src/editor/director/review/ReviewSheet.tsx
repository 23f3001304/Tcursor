import { AnimatePresence } from "motion/react";
import { Spin } from "../../controls/surfaces/Spin";
import { ReviewItem } from "./ReviewItem";
import { acceptedIds, summary } from "./reviewState";
import { engineDisplayName } from "../engineName";
import type { AiRun } from "../../../shared/aiRun";

function provenance(run: AiRun): string {
  const secs = `${(run.elapsed_ms / 1000).toFixed(1)}s`;
  if (run.frames === 0) return secs;
  return `${run.frames} frame${run.frames === 1 ? "" : "s"}, ${secs}`;
}

export function ReviewSheet({
  run,
  skipped,
  applying,
  previewId,
  onToggle,
  onPreview,
  onApply,
  onDiscard,
}: {
  run: AiRun;
  skipped: ReadonlySet<string>;
  applying: boolean;
  previewId: string | null;
  onToggle: (id: string) => void;
  onPreview: (id: string) => void;
  onApply: () => void;
  onDiscard: () => void;
}) {
  const accepted = acceptedIds(run, skipped).length;
  const empty = run.proposals.length === 0;
  return (
    <div className="e-grp e-rev">
      <div className="e-rev-head">
        <span className="e-rev-model" title={run.model}>
          {engineDisplayName(run.model)}
        </span>
        <span className="e-rev-prov">{provenance(run)}</span>
      </div>
      <p className={empty ? "e-hintline" : "e-rev-sum"}>{summary(run, skipped)}</p>

      {!empty && (
        <ul className="e-rev-list">
          <AnimatePresence initial={false}>
            {run.proposals.map((p) => (
              <ReviewItem
                key={p.id}
                p={p}
                accepted={!skipped.has(p.id)}
                previewing={previewId === p.id}
                disabled={applying}
                onToggle={onToggle}
                onPreview={onPreview}
              />
            ))}
          </AnimatePresence>
        </ul>
      )}

      <div className="e-rev-foot">
        {!empty && (
          <button
            type="button"
            className="e-run e-rev-apply"
            onClick={onApply}
            disabled={applying || accepted === 0}
            title={accepted === 0 ? "Accept at least one edit to apply" : undefined}
          >
            {applying ? (
              <>
                <Spin size={16} />
                Applying...
              </>
            ) : (
              `Apply ${accepted} edit${accepted === 1 ? "" : "s"}`
            )}
          </button>
        )}
        <button type="button" className="e-ghostbtn e-rev-discard" onClick={onDiscard} disabled={applying}>
          Discard
        </button>
      </div>
    </div>
  );
}
