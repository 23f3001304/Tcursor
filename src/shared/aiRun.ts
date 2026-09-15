import type { EditOp } from "./edit";

export type AiProposalKind = "zoom" | "layout" | "spotlight" | "trim" | "cut" | "speed";

export interface AiProposal {
  id: string;
  kind: AiProposalKind;
  why: string;
  at_ms: number;
  dur_ms: number;
  rect: [number, number, number, number] | null;
  ops: EditOp[];
}

export interface AiRun {
  model: string;
  vision: boolean;
  frames: number;
  elapsed_ms: number;
  proposals: AiProposal[];
}

export const AI_NEW_ID = "$new";
