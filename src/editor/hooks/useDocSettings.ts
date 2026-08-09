import { useCallback } from "react";
import { saveEdit } from "../../lib/ipc";
import type { EditDoc } from "../../lib/edit";

/** Settings-only doc writes, split out of `Editor` (bulk `settings` patches, as opposed to
 *  `applyOp`'s per-`EditOp` mutations). Both flavors share one underlying write (`write`):
 *  `saveDocSettings` records an undo step first - the normal path for every settings-editing
 *  panel (background, cursor, camera, captions, audio, the AI model picker). `onAutoModel` skips
 *  `record()` - it's used ONLY by `AiPanel`'s mount-time "default to an installed Ollama model"
 *  effect, so landing on that default the instant the panel opens never pushes a phantom undo
 *  step or an unasked-for disk write purely from mounting. Memoized on `doc` (not every render,
 *  e.g. every `timeMs` tick during playback) so that effect - keyed on `onAutoModel`'s identity -
 *  doesn't needlessly re-fire while nothing about the doc has actually changed. */
export function useDocSettings(
  folder: string,
  doc: EditDoc | null,
  setDoc: (d: EditDoc) => void,
  record: (current: EditDoc) => void,
  setRev: (fn: (r: number) => number) => void,
) {
  const write = useCallback(async (nextSettings: EditDoc["settings"]) => {
    if (!doc) return;
    const newDoc = { ...doc, settings: nextSettings };
    setDoc(newDoc);
    await saveEdit(folder, newDoc);
    setRev((r) => r + 1);
  }, [doc, folder, setDoc, setRev]);

  const saveDocSettings = useCallback((nextSettings: EditDoc["settings"]) => {
    if (doc) record(doc); // snapshot for undo
    return write(nextSettings);
  }, [doc, record, write]);

  const onAutoModel = useCallback((v: string) => {
    if (doc) void write({ ...doc.settings, ai_model: v });
  }, [doc, write]);

  return { saveDocSettings, onAutoModel };
}
