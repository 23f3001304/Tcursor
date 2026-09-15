import { useCallback, type RefObject } from "react";
import { saveEdit } from "../../../shared/ipc";
import type { EditDoc } from "../../../shared/edit";
import type { RecordToken } from "./useEditHistory";

export function useDocSettings(
  folder: string,
  docRef: RefObject<EditDoc | null>,
  setDoc: (d: EditDoc) => void,
  record: (current: EditDoc) => RecordToken,
  setRev: (fn: (r: number) => number) => void,
  enqueue: <T>(fn: () => Promise<T>) => Promise<T>,
) {
  const write = useCallback(
    (nextSettings: EditDoc["settings"], recordUndo: boolean) =>
      enqueue(async () => {
        const doc = docRef.current;
        if (!doc) return;
        if (recordUndo) record(doc);
        const newDoc = { ...doc, settings: nextSettings };
        setDoc(newDoc);
        docRef.current = newDoc;
        try {
          await saveEdit(folder, newDoc);
        } catch {}
        setRev((r) => r + 1);
      }),
    [docRef, folder, record, setDoc, setRev, enqueue],
  );

  const saveDocSettings = useCallback(
    (nextSettings: EditDoc["settings"]) => write(nextSettings, true),
    [write],
  );

  const onAutoModel = useCallback(
    (v: string) => {
      const doc = docRef.current;
      if (doc) void write({ ...doc.settings, ai_model: v }, false);
    },
    [docRef, write],
  );

  return { saveDocSettings, onAutoModel };
}
