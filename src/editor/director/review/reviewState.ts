import type { EditOp } from "../../../shared/edit";
import type { AiProposal, AiRun } from "../../../shared/aiRun";

export function toggle(skipped: ReadonlySet<string>, id: string): ReadonlySet<string> {
  const next = new Set(skipped);
  if (!next.delete(id)) next.add(id);
  return next;
}

export function acceptedIds(run: AiRun, skipped: ReadonlySet<string>): string[] {
  return run.proposals.filter((p) => !skipped.has(p.id)).map((p) => p.id);
}

export function orderedOps(run: AiRun, skipped: ReadonlySet<string>): EditOp[] {
  return run.proposals.filter((p) => !skipped.has(p.id)).flatMap((p) => p.ops);
}

export function summary(run: AiRun, skipped: ReadonlySet<string>): string {
  const n = run.proposals.length;
  if (n === 0) return "Nothing worth editing was found in this clip.";
  return `${n} edit${n === 1 ? "" : "s"}, ${acceptedIds(run, skipped).length} accepted`;
}

export function previewTarget(
  run: AiRun,
  id: string,
): { tMs: number; rect: [number, number, number, number] | null } | null {
  const p: AiProposal | undefined = run.proposals.find((x) => x.id === id);
  return p ? { tMs: p.at_ms, rect: p.rect } : null;
}
