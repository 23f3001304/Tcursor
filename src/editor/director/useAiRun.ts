import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import type { EditDoc } from "../../shared/edit";
import { aiPropose, applyEditOp } from "../../shared/ipc";
import type { AiProposal, AiRun } from "../../shared/aiRun";
import type { DirectorPointerHandle } from "./DirectorPointer";
import { anchorPoint, timelinePointForMs } from "./targets";
import { planStep, dwellSettle } from "./choreography";
import { previewTarget, toggle } from "./review/reviewState";
import { applyRun } from "./applyOps";

const sleep = (ms: number) => new Promise<void>((res) => setTimeout(res, ms));

const OUTLINE_MS = 2500;

export type Rect = [number, number, number, number];
export interface AiProgress {
  step: number;
  total: number;
}

export async function planOrCancel<T>(
  cancelRef: { current: boolean },
  fetch: () => Promise<T>,
): Promise<T | null> {
  const result = await fetch();
  return cancelRef.current ? null : result;
}

export interface AiRunIo {
  folder: string;
  docRef: RefObject<EditDoc | null>;
  dur: number;
  enqueue: <T>(fn: () => Promise<T>) => Promise<T>;
  record: (d: EditDoc) => void;
  setDoc: (d: EditDoc) => void;
  bumpRev: () => void;
  onSeek: (ms: number) => void;
  setPlaying: (p: boolean) => void;
}

export function useAiRun(io: AiRunIo) {
  const ioRef = useRef(io);
  ioRef.current = io;
  const pointerRef = useRef<DirectorPointerHandle>(null);
  const cancelRef = useRef(false);
  const busyRef = useRef(false);
  const [aiRun, setAiRun] = useState<AiRun | null>(null);
  const [skipped, setSkipped] = useState<ReadonlySet<string>>(() => new Set());
  const [previewId, setPreviewId] = useState<string | null>(null);
  const [outline, setOutline] = useState<Rect | null>(null);
  const [planning, setPlanning] = useState(false);
  const [applying, setApplying] = useState(false);
  const [replaying, setReplaying] = useState(false);
  const [progress, setProgress] = useState<AiProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  const requestCancel = useCallback(() => {
    cancelRef.current = true;
  }, []);
  const toggleItem = useCallback((id: string) => setSkipped((s) => toggle(s, id)), []);
  const discard = useCallback(() => {
    setAiRun(null);
    setSkipped(new Set());
    setPreviewId(null);
    setOutline(null);
  }, []);
  const preview = useCallback(
    (id: string) => {
      const t = aiRun && previewTarget(aiRun, id);
      if (!t) return;
      setPreviewId(id);
      ioRef.current.setPlaying(false);
      ioRef.current.onSeek(t.tMs);
      setOutline(t.rect);
    },
    [aiRun],
  );
  useEffect(() => {
    if (!outline) return;
    const t = setTimeout(() => {
      setOutline(null);
      setPreviewId(null);
    }, OUTLINE_MS);
    return () => clearTimeout(t);
  }, [outline]);

  const run = useCallback(async () => {
    const { folder, docRef } = ioRef.current;
    const doc = docRef.current;
    if (!doc || busyRef.current) return;
    busyRef.current = true;
    cancelRef.current = false;
    setPlanning(true);
    setError(null);
    try {
      const got = await planOrCancel(cancelRef, () => aiPropose(folder, doc.settings.ai_model || undefined));
      if (got) {
        setAiRun(got);
        setSkipped(new Set());
        setPreviewId(null);
        setOutline(null);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setPlanning(false);
      busyRef.current = false;
    }
  }, []);

  const replay = useCallback(async (items: AiProposal[]) => {
    setReplaying(true);
    setProgress({ step: 0, total: items.length });
    try {
      const { dwellMs, settleMs } = dwellSettle(items.length);
      await sleep(dwellMs);
      const wand = anchorPoint("wand");
      if (wand) await pointerRef.current?.moveTo(wand.x, wand.y);
      await pointerRef.current?.press();
      for (let i = 0; i < items.length && !cancelRef.current; i++) {
        const track = document.querySelector<HTMLElement>(".e-tlbody");
        const plan = planStep(items[i].ops[0]);
        if (track && plan.kind !== "none" && plan.ms !== undefined) {
          const pt = timelinePointForMs(track, plan.ms, ioRef.current.dur, plan.lane);
          await pointerRef.current?.moveTo(pt.x, pt.y);
          await sleep(dwellMs);
          if (plan.pressAfterMove) await pointerRef.current?.press();
          await sleep(settleMs);
        }
        setProgress({ step: i + 1, total: items.length });
      }
    } finally {
      setReplaying(false);
      setProgress(null);
    }
  }, []);

  const apply = useCallback(async () => {
    if (!aiRun || busyRef.current) return;
    busyRef.current = true;
    cancelRef.current = false;
    setApplying(true);
    setError(null);
    const accepted = aiRun.proposals.filter((p) => !skipped.has(p.id));
    try {
      const landed = await ioRef.current.enqueue(() => {
        const { folder, record, setDoc, docRef } = ioRef.current;
        return applyRun(aiRun, skipped, { record, setDoc, docRef, applyOp: (op) => applyEditOp(folder, op) });
      });
      if (landed > 0) ioRef.current.bumpRev();
      discard();
      if (landed > 0 && ioRef.current.docRef.current?.settings.ui.ai_choreography) await replay(accepted);
    } catch (e) {
      setError(String(e));
    } finally {
      setApplying(false);
      busyRef.current = false;
    }
  }, [aiRun, skipped, discard, replay]);

  return {
    pointerRef,
    cancelRef,
    requestCancel,
    run,
    apply,
    discard,
    toggleItem,
    preview,
    aiRun,
    skipped,
    previewId,
    outline,
    planning,
    applying,
    replaying,
    progress,
    error,
    running: planning || applying || replaying,
  };
}
