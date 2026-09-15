import { useCallback, type RefObject } from "react";
import type { EditDoc, EditOp } from "../../../shared/edit";
import { detectSilences } from "../../../shared/ipc";

export function newSilences(
  spans: [number, number][],
  cuts: { start_ms: number; end_ms: number }[],
): [number, number][] {
  return spans.filter(([a, b]) => !cuts.some((c) => c.start_ms <= a && b <= c.end_ms));
}

export function silenceToast(spans: [number, number][]): string {
  if (spans.length === 0) return "No silences found";
  const ms = spans.reduce((total, [a, b]) => total + (b - a), 0);
  return `Removed ${spans.length} silence${spans.length === 1 ? "" : "s"}, ${(ms / 1000).toFixed(1)} s`;
}

export function useSilences(
  folder: string,
  docRef: RefObject<EditDoc | null>,
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
  toast: (msg: string) => void,
) {
  return useCallback(async () => {
    try {
      const spans = newSilences(await detectSilences(folder), docRef.current?.cuts ?? []);
      if (spans.length) await applyOp({ op: "add_cuts", spans });
      toast(silenceToast(spans));
    } catch (e) {
      toast(typeof e === "string" ? e : "Could not scan for silences");
    }
  }, [folder, docRef, applyOp, toast]);
}
