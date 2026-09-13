import { useCallback, useRef, useState, type Dispatch, type RefObject, type SetStateAction } from "react";
import type { EditDoc } from "../../lib/edit";
import { resolveTrim } from "../../lib/edit";
import { aiPlan, applyEditOp, type AiStep } from "../../lib/ipc";
import type { DirectorPointerHandle } from "./DirectorPointer";
import { anchorPoint, timelinePointForMs, pillPoint, type Lane } from "./targets";
import { planStep, dwellSettle } from "./choreography";

const sleep = (ms: number) => new Promise<void>((res) => setTimeout(res, ms));

export interface DirectorProgress { step: number; total: number }

/** Races `fetchPlan` against `cancelRef`: resolves to the fetched plan normally, or to `null` if
 *  `cancelRef.current` was set (Esc/scrim/Stop-pill) any time before the fetch settled - the
 *  fetch itself is NOT aborted (the HTTP call runs to completion server-side; only the app-side
 *  reaction to it is skipped). Plain function, no React, so the planning-phase cancel path is
 *  unit-testable without mounting the hook - see `useDirector.test.ts`. */
export async function planOrCancel<T>(cancelRef: { current: boolean }, fetchPlan: () => Promise<T>): Promise<T | null> {
  const result = await fetchPlan();
  return cancelRef.current ? null : result;
}

/** `planOrCancel`, then - ONLY when it actually produced steps (not cancelled, not rejected) -
 *  calls `record()` immediately before returning them. This is what keeps the undo boundary out
 *  of the undo stack for a pass that never applies anything: `record()` used to run unconditionally
 *  up front, so cancelling during a slow (first-run-model-load) planning fetch still left a no-op
 *  snapshot behind - the Undo button would light up over nothing. Plain function, no React - see
 *  `useDirector.test.ts`. */
export async function fetchStepsToReveal<T>(
  cancelRef: { current: boolean }, fetchPlan: () => Promise<T>, record: () => void,
): Promise<T | null> {
  const steps = await planOrCancel(cancelRef, fetchPlan);
  if (steps === null) return null;
  record();
  return steps;
}

/** Owns the AI director's fake-pointer handle plus the run's live progress/cancel state, and
 *  drives the choreographed reveal AROUND `Editor.tsx`'s existing queued-pass correctness
 *  machinery (the F2 fix, commit d08c71e - single `record(doc0)`, `running` set pre-enqueue,
 *  per-step raw `applyEditOp`/`setDoc`/`rev`, all inside ONE `enqueue(...)` call). Kept out of
 *  `Editor.tsx` (already at its line budget) so the choreography doesn't grow the file that owns
 *  undo/queue correctness - `run` below performs EXACTLY the same statements in the EXACT same
 *  order the pre-choreography `onRun` did (see `Editor.md`), just from here instead. */
export function useDirector() {
  const pointerRef = useRef<DirectorPointerHandle>(null);
  const cancelRef = useRef(false);
  const [progress, setProgress] = useState<DirectorProgress | null>(null);
  // True only while `aiPlan`'s fetch is in flight (the Rust command now runs off the main thread,
  // but a first-run model load can still take minutes) - lets the Stop pill show an explicit
  // "asking the model" state distinct from the step-by-step reveal's "step k/N".
  const [planning, setPlanning] = useState(false);

  // `useCallback`'d (render hygiene pass, all the way down to `aimAndPress`) so `run`'s own
  // identity is stable across renders - `Editor.tsx`'s `onRun` wraps it directly, and `onRun` is
  // handed to `Transport`/`EditorPanels` (both `React.memo`'d). Every dependency below bottoms out
  // in a ref or a `useState` setter, both permanently stable, so this chain holds across every
  // render, not just ticks.
  const requestCancel = useCallback(() => { cancelRef.current = true; }, []);

  // Per-step choreography: aim, dwell, and (when the plan calls for it) press, all BEFORE the
  // step's real op is applied - unchanged call order from before this task, just wrapped.
  const aimAndPress = useCallback(async (track: HTMLElement, dur: number, ms: number, lane: Lane, dwellMs: number, press: boolean) => {
    const pt = timelinePointForMs(track, ms, dur, lane);
    await pointerRef.current?.moveTo(pt.x, pt.y);
    await sleep(dwellMs);
    if (press) await pointerRef.current?.press();
  }, []);

  /** Reveals one already-fetched plan, choreographing `pointerRef` around each step's real
   *  `applyStep` (identical to the old inline loop's body - apply, `setDoc`, `rev`, narrate,
   *  scrub). Returns the number of steps actually applied (< `steps.length` iff cancelled
   *  mid-run) - the CURRENT step always finishes; cancel only skips the ones after it. */
  const reveal = useCallback(async (
    steps: AiStep[], dur: number, docBefore: EditDoc, applyStep: (step: AiStep) => Promise<EditDoc>,
  ): Promise<number> => {
    // NOT reset here - `run` resets it once, before the planning fetch, so a cancel that lands
    // during that fetch (caught by `planOrCancel`) survives to be seen below instead of being
    // silently wiped out the moment the reveal phase starts.
    setProgress({ step: 0, total: steps.length });
    // Re-measure the wand anchor (DirectorPointer already snapped there on mount, but the plan
    // fetch this waited on can take seconds - a resize in that window shouldn't leave the press
    // aimed at a stale button position) then press it: the run visibly starts from the button.
    const wand = anchorPoint("wand");
    if (wand) await pointerRef.current?.moveTo(wand.x, wand.y);
    await pointerRef.current?.press();
    let current = docBefore;
    const { dwellMs, settleMs } = dwellSettle(steps.length);

    for (let i = 0; i < steps.length; i++) {
      if (cancelRef.current) return i;
      const step = steps[i];
      const plan = planStep(step.op);
      const track = document.querySelector<HTMLElement>(".e-tlbody"); // re-measured every step

      if (plan.kind === "aim-timeline" && track && plan.ms !== undefined) {
        await aimAndPress(track, dur, plan.ms, plan.lane, dwellMs, plan.pressAfterMove);
      } else if (plan.kind === "drag-trim" && step.op.op === "set_trim" && track) {
        const before = resolveTrim(current.trim, dur);
        const after = resolveTrim({ in_ms: step.op.in_ms, out_ms: step.op.out_ms }, dur);
        if (after.inMs !== before.inMs) {
          await aimAndPress(track, dur, before.inMs, "trim-in", dwellMs, true);
          const tgt = timelinePointForMs(track, after.inMs, dur, "trim-in");
          await pointerRef.current?.moveTo(tgt.x, tgt.y);
        }
        if (after.outMs !== before.outMs) {
          await aimAndPress(track, dur, before.outMs, "trim-out", dwellMs, true);
          const tgt = timelinePointForMs(track, after.outMs, dur, "trim-out");
          await pointerRef.current?.moveTo(tgt.x, tgt.y);
        }
      }

      let newDoc: EditDoc;
      if (plan.kind === "sweep-lane" && track) {
        const rows = track.querySelectorAll<HTMLElement>(".e-zoomrow");
        const row = rows[rows.length - 1] ?? track;
        const r = row.getBoundingClientRect(), t = track.getBoundingClientRect();
        [, newDoc] = await Promise.all([
          pointerRef.current?.sweep(t.left, t.right, r.top + r.height / 2) ?? Promise.resolve(),
          applyStep(step),
        ]);
      } else {
        newDoc = await applyStep(step); // the op applies once, after every glide above has settled
      }

      if (plan.kind === "aim-timeline" && track) {
        const newId = newDoc.zooms[newDoc.zooms.length - 1]?.id;
        const pp = newId ? pillPoint(track, newId) : null;
        if (pp) await pointerRef.current?.moveTo(pp.x, pp.y); // settle onto the REAL pill, not the guess
      }
      await sleep(settleMs);
      current = newDoc;
      setProgress({ step: i + 1, total: steps.length });
    }
    return steps.length;
  }, [aimAndPress]);

  /** Fetches the plan and reveals it - the whole thing is meant to run inside Editor's single
   *  `enqueue(...)` call (see the module doc above); `record`/`setDoc`/etc. are handed in rather
   *  than imported so this hook never touches Editor's state directly. */
  const run = useCallback(async (
    folder: string, docRef: RefObject<EditDoc | null>, record: (d: EditDoc) => void, dur: number,
    setDoc: (d: EditDoc) => void, setRev: Dispatch<SetStateAction<number>>, setAiLog: Dispatch<SetStateAction<string[]>>,
    setAiError: (e: string | null) => void, setPlaying: (p: boolean) => void, setTimeMs: (ms: number) => void,
  ): Promise<void> => {
    const doc0 = docRef.current;
    if (!doc0) return;
    cancelRef.current = false; // the ONE reset for the whole pass - covers both the fetch below and reveal()
    setPlanning(true);
    let steps: AiStep[] | null;
    try {
      // `fetchStepsToReveal` - not a bare `await aiPlan(...)` - so (a) a cancel that lands while
      // this fetch is in flight (Esc/scrim/Stop pill, all via `requestCancel` -> `cancelRef`) is
      // still seen once it resolves, even though the HTTP call itself already ran to completion
      // server-side, and (b) `record(doc0)` only fires once a reveal is actually about to happen -
      // NOT up front - so a cancelled-during-planning pass leaves no spurious undo snapshot behind.
      steps = await fetchStepsToReveal(cancelRef, () => aiPlan(folder, doc0.settings.ai_model || undefined), () => record(doc0));
    } catch (e) {
      setPlanning(false);
      setAiError(String(e));
      return;
    }
    setPlanning(false);
    if (steps === null) {
      // Bail before applying anything (and before `record` ever ran); `run`'s Promise still
      // resolves normally, so Editor's `enqueue(...)` (and its `.finally(() =>
      // setRunning(false))`) release right away.
      setAiLog((l) => [...l, "Stopped before planning finished"]);
      return;
    }
    try {
      const kept = await reveal(steps, dur, doc0, async (step) => {
        const d = await applyEditOp(folder, step.op); // raw apply - the ONE record() above covers the whole pass
        setDoc(d); docRef.current = d; setRev((r) => r + 1); // (review round 1, Important 2) - see Editor.md's M1 note
        setAiLog((l) => [...l, step.label]);
        if (step.op.op === "add_zoom_full") { setPlaying(false); setTimeMs(step.op.at_ms); }
        return d;
      });
      if (kept < steps.length) setAiLog((l) => [...l, `Stopped, kept ${kept} edit${kept === 1 ? "" : "s"}`]);
      else {
        const zoomN = steps.filter((s) => s.op.op === "add_zoom_full").length;
        setAiLog((l) => [...l, `✓ Done · ${zoomN} zoom${zoomN === 1 ? "" : "s"}`]);
      }
    } catch (e) { setAiError(String(e)); }
  }, [reveal]);

  return { pointerRef, progress, planning, cancelRef, requestCancel, run };
}
