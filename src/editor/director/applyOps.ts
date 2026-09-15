import type { EditDoc, EditOp } from "../../shared/edit";
import { AI_NEW_ID, type AiRun } from "../../shared/aiRun";
import { orderedOps } from "./review/reviewState";

export function substituteNewId(op: EditOp, newId: string): EditOp {
  return "id" in op && op.id === AI_NEW_ID ? { ...op, id: newId } : op;
}

export function newRegionId(before: EditDoc, after: EditDoc): string | null {
  const lists = ["zooms", "effects", "layout"] as const;
  for (const k of lists) {
    const seen = new Set(before[k].map((r) => r.id));
    const fresh = after[k].find((r) => !seen.has(r.id));
    if (fresh) return fresh.id;
  }
  return null;
}

export interface ApplyIo {
  record: (d: EditDoc) => void;
  applyOp: (op: EditOp) => Promise<EditDoc>;
  setDoc: (d: EditDoc) => void;
  docRef: { current: EditDoc | null };
}

export async function applyRun(run: AiRun, skipped: ReadonlySet<string>, io: ApplyIo): Promise<number> {
  const ops = orderedOps(run, skipped);
  const doc0 = io.docRef.current;
  if (ops.length === 0 || !doc0) return 0;
  io.record(doc0);
  let prev = doc0;
  let created: string | null = null;
  let landed = 0;
  for (const raw of ops) {
    let op = raw;
    if ("id" in raw && raw.id === AI_NEW_ID) {
      if (!created) continue;
      op = substituteNewId(raw, created);
    }
    const next = await io.applyOp(op);
    created = newRegionId(prev, next);
    io.setDoc(next);
    io.docRef.current = next;
    prev = next;
    landed++;
  }
  return landed;
}
