import { type ReactNode } from "react";
import { motion, useReducedMotion } from "motion/react";
import { IconTrash, IconX } from "@tabler/icons-react";

export const PRESS = { scale: 0.96 };
export const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

export const secOf = (ms: number) => +(ms / 1000).toFixed(2);

export const secText = (ms: number) => `${(Math.max(0, ms) / 1000).toFixed(2)}s`;

export const spanRange = (startMs: number, endMs: number) => `${secText(startMs)} to ${secText(endMs)}`;

export type InspectorKind = "zoom" | "fx" | "layout" | "cam" | "cut" | "speed" | "caption" | "clip";

export function InspectorShell({ kind, children }: { kind: InspectorKind; children: ReactNode }) {
  return <div className={`e-panel e-insp e-insp-${kind}`}>{children}</div>;
}

export function InspectorHeader({
  title,
  range,
  deleteLabel,
  onDelete,
  onClose,
  thumb,
}: {
  title: string;
  range: string;
  deleteLabel: string;
  onDelete: () => void;
  onClose: () => void;
  thumb?: ReactNode;
}) {
  const still = useReducedMotion();
  const tap = still ? undefined : PRESS;
  return (
    <header className="e-ihead">
      <div className="e-ihead-top">
        <span className="e-idot" aria-hidden="true" />
        <h2>{title}</h2>
        {thumb}
        <span className="e-ihead-sp" />
        <motion.button
          type="button"
          className="e-ihicon del"
          title={deleteLabel}
          aria-label={deleteLabel}
          onClick={onDelete}
          whileTap={tap}
          transition={PRESS_SPRING}
        >
          <IconTrash size={15} />
        </motion.button>
        <motion.button
          type="button"
          className="e-ihicon"
          title="Deselect"
          aria-label="Deselect"
          onClick={onClose}
          whileTap={tap}
          transition={PRESS_SPRING}
        >
          <IconX size={15} />
        </motion.button>
      </div>
      <p className="e-ihead-range" title={range}>
        {range}
      </p>
    </header>
  );
}

export function Section({
  title,
  value,
  children,
}: {
  title: string;
  value?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="e-isec">
      <div className="e-isec-head">
        <h3>{title}</h3>
        {value !== undefined && <span className="e-isec-val">{value}</span>}
      </div>
      {children}
    </section>
  );
}

export const Hint = ({ children }: { children: ReactNode }) => <p className="e-ihint">{children}</p>;
