import type { Caption, EditDoc, EditOp } from "../../shared/edit";
import { CaptionTextField } from "./CaptionTextField";
import { insideSpan, nextCaption, splitPoints } from "./captionEdit";
import { Hint, InspectorHeader, InspectorShell, Section, secText, spanRange } from "./InspectorShape";
import { TimingRow } from "./InspectorRows";

export function CaptionInspector({
  caption,
  captions,
  dur,
  timeMs,
  onApply,
  onClose,
}: {
  caption: Caption;
  captions: Caption[];
  dur: number;
  timeMs: number;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onClose: () => void;
}) {
  const upd = (patch: { start_ms?: number; end_ms?: number; text?: string }) =>
    void onApply({ op: "update_caption", id: caption.id, ...patch });
  const points = splitPoints(caption);
  const next = nextCaption(captions, caption.id);
  const playheadSplit = points.length === 0 && insideSpan(caption, timeMs);

  return (
    <InspectorShell kind="caption">
      <InspectorHeader
        title="Caption"
        range={spanRange(caption.start_ms, caption.end_ms)}
        deleteLabel="Remove caption"
        onClose={onClose}
        onDelete={() => {
          void onApply({ op: "remove_caption", id: caption.id });
          onClose();
        }}
      />

      <Section title="Text" value={caption.words.length > 0 ? `${caption.words.length} words` : "typed"}>
        <CaptionTextField value={caption.text} onCommit={(text) => upd({ text })} />
        <Hint>
          {caption.words.length === 0
            ? "This caption has no word timings, so the word-by-word highlight stays off for it."
            : "Enter commits, Escape puts it back. Retyping the words drops this caption's highlight timings, because a highlight on stale timings lights the wrong word."}
        </Hint>
      </Section>

      <Section title="Timing" value={secText(caption.end_ms - caption.start_ms)}>
        <TimingRow
          startMs={caption.start_ms}
          endMs={caption.end_ms}
          durMs={dur}
          onStart={(start_ms) => upd({ start_ms })}
          onEnd={(end_ms) => upd({ end_ms })}
        />
        <Hint>Drag the pill on the Captions lane to move it.</Hint>
      </Section>

      <Section title="Split">
        {points.length > 0 ? (
          <>
            <div className="e-capwords" role="group" aria-label="Split before a word">
              {points.map((p) => (
                <button
                  key={p.atMs}
                  type="button"
                  className="e-capword"
                  title={`Split before "${p.label}", at ${secText(p.atMs)}`}
                  onClick={() => void onApply({ op: "split_caption", id: caption.id, at_ms: p.atMs })}
                >
                  {p.label}
                </button>
              ))}
            </div>
            <Hint>Pick the word the second caption should start with.</Hint>
          </>
        ) : (
          <>
            <button
              type="button"
              className="e-ghostbtn"
              disabled={!playheadSplit}
              onClick={() => void onApply({ op: "split_caption", id: caption.id, at_ms: Math.round(timeMs) })}
            >
              Split at the playhead
            </button>
            <Hint>
              {playheadSplit
                ? "With no word timings the cut lands at the nearest space in the text."
                : "Move the playhead inside this caption to split it."}
            </Hint>
          </>
        )}
      </Section>

      <Section title="Merge">
        <button
          type="button"
          className="e-ghostbtn"
          disabled={next === null}
          title={
            next
              ? `Join with "${next.text}"`
              : "This is the last caption, so there is nothing after it to join."
          }
          onClick={() => void onApply({ op: "merge_captions", id: caption.id })}
        >
          Merge with next
        </button>
        <Hint>
          {next
            ? `The next caption runs to ${secText(next.end_ms)}; both texts and both word lists become one.`
            : "This is the last caption, so there is nothing after it to join."}
        </Hint>
      </Section>
    </InspectorShell>
  );
}
