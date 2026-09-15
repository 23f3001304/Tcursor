import { useCallback, useRef, useState } from "react";
import { applyEditOp, getEdit } from "../../../shared/ipc";
import type { EditDoc, EditOp } from "../../../shared/edit";
import { createQueue } from "../../util/opQueue";
import { useUndoToast } from "../../shell/dialogs/Toast";
import { useTimeMap } from "../stage/useTimeMap";
import { useEditorData } from "./useEditorData";
import { useEditHistory } from "./useEditHistory";
import { useExportState } from "./useExportState";
import { useDocSettings } from "./useDocSettings";

export function useEditorSession(folder: string, quality: number, vidDurMs: number) {
  const [rev, setRev] = useState(0);
  const { msg: toast, onSwap, push: pushToast, dismiss: dismissToast } = useUndoToast();
  const enqueue = useRef(createQueue()).current;
  const data = useEditorData(folder, rev, quality);
  const { doc, setDoc } = data;
  const exportState = useExportState(pushToast);
  const docRef = useRef(doc);
  docRef.current = doc;

  const dur = vidDurMs > 0 ? vidDurMs : (doc?.trim.out_ms ?? 0);
  const { map, outDoc } = useTimeMap(doc, dur);
  const bumpRev = useCallback(() => setRev((r) => r + 1), []);
  const { record, unrecord, undo, redo, canUndo, canRedo } = useEditHistory(
    folder,
    docRef,
    setDoc,
    bumpRev,
    enqueue,
    onSwap,
  );

  const applyOp = useCallback(
    (op: EditOp): Promise<EditDoc | null> =>
      enqueue(async () => {
        const token = docRef.current ? record(docRef.current) : null;
        try {
          const d = await applyEditOp(folder, op);
          setDoc(d);
          docRef.current = d;
          if (!op.op.endsWith("_effect")) setRev((r) => r + 1);
          return d;
        } catch {
          if (token) unrecord(token);
          return null;
        }
      }),
    [enqueue, docRef, record, unrecord, folder, setDoc, setRev],
  );
  const reloadDoc = useCallback(() => {
    void enqueue(async () => {
      if (docRef.current) record(docRef.current);
      const d = await getEdit(folder).catch(() => null);
      if (d) {
        setDoc(d);
        docRef.current = d;
        setRev((r) => r + 1);
      }
    });
  }, [enqueue, docRef, record, folder, setDoc, setRev]);
  const { saveDocSettings, onAutoModel } = useDocSettings(folder, docRef, setDoc, record, setRev, enqueue);

  return {
    ...data,
    docRef,
    dur,
    map,
    outDoc,
    enqueue,
    record,
    bumpRev,
    applyOp,
    reloadDoc,
    undo,
    redo,
    canUndo,
    canRedo,
    exportState,
    saveDocSettings,
    onAutoModel,
    toast,
    pushToast,
    dismissToast,
  };
}
