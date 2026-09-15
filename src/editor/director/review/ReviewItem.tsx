import type { ComponentType } from "react";
import { motion, useReducedMotion } from "motion/react";
import {
  IconCut,
  IconEye,
  IconLayoutGrid,
  IconPlayerTrackNext,
  IconScissors,
  IconWand,
  IconZoomIn,
} from "@tabler/icons-react";
import { Switch } from "../../controls/Controls";
import { fmt } from "../../timeline/model/time";
import type { AiProposal, AiProposalKind } from "../../../shared/aiRun";

export const HINT_MOTION = {
  initial: { opacity: 0, y: -4 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -4 },
  transition: { duration: 0.14 },
};

type Glyph = ComponentType<{ size?: number }>;

const KIND_ICON: Record<AiProposalKind, Glyph> = {
  zoom: IconZoomIn,
  layout: IconLayoutGrid,
  spotlight: IconWand,
  trim: IconScissors,
  cut: IconCut,
  speed: IconPlayerTrackNext,
};
const KIND_LABEL: Record<AiProposalKind, string> = {
  zoom: "Zoom",
  layout: "Layout",
  spotlight: "Spotlight",
  trim: "Trim",
  cut: "Cut",
  speed: "Speed",
};

export function ReviewItem({
  p,
  accepted,
  previewing,
  disabled,
  onToggle,
  onPreview,
}: {
  p: AiProposal;
  accepted: boolean;
  previewing: boolean;
  disabled: boolean;
  onToggle: (id: string) => void;
  onPreview: (id: string) => void;
}) {
  const still = useReducedMotion();
  const Icon = KIND_ICON[p.kind] ?? IconWand;
  const what = `${KIND_LABEL[p.kind] ?? "Edit"} at ${fmt(p.at_ms)}`;
  return (
    <motion.li
      className={`e-rev-item${accepted ? "" : " off"}${previewing ? " on" : ""}`}
      {...(still ? {} : HINT_MOTION)}
    >
      <Switch
        on={accepted}
        onChange={() => onToggle(p.id)}
        disabled={disabled}
        ariaLabel={`Accept ${what}`}
        title={accepted ? "Accepted" : "Skipped"}
      />
      <Icon size={14} />
      <span className="e-rev-time">{fmt(p.at_ms)}</span>
      <span className="e-rev-why" title={`${what}: ${p.why}`}>
        {p.why}
      </span>
      <button
        type="button"
        className="e-rev-eye"
        onClick={() => onPreview(p.id)}
        disabled={disabled}
        title="Preview this edit"
        aria-label={`Preview ${what}`}
      >
        <IconEye size={14} />
      </button>
    </motion.li>
  );
}
