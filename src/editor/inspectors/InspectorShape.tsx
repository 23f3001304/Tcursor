import { useId, type ReactNode } from "react";
import { motion, useReducedMotion } from "motion/react";
import { IconTrash } from "@tabler/icons-react";
import { NumberField } from "../controls/Controls";

const PRESS = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };
const GLIDE = { type: "spring" as const, stiffness: 500, damping: 40 };

export const secOf = (ms: number) => +(ms / 1000).toFixed(2);

export function clock(ms: number): string {
  const t = Math.max(0, ms) / 1000;
  const m = Math.floor(t / 60);
  return `${m}:${(t - m * 60).toFixed(1).padStart(4, "0")}`;
}

export const spanLede = (startMs: number, endMs: number) =>
  `${clock(startMs)} to ${clock(endMs)}, ${((endMs - startMs) / 1000).toFixed(1)} s`;

export type InspectorKind = "zoom" | "fx" | "layout" | "cam" | "cut" | "speed";

export function InspectorShell({ kind, children }: { kind: InspectorKind; children: ReactNode }) {
  return <div className={`e-panel e-insp e-insp-${kind}`}>{children}</div>;
}

export function Section({ title, value, children }: { title: string; value?: ReactNode; children: ReactNode }) {
  return (
    <section className="e-isec">
      <div className="e-isec-head"><h3>{title}</h3>{value !== undefined && <span className="e-isec-val">{value}</span>}</div>
      {children}
    </section>
  );
}

export const Hint = ({ children }: { children: ReactNode }) => <p className="e-ihint">{children}</p>;

export function TimingRow({ startMs, endMs, durMs, onStart, onEnd }: {
  startMs: number; endMs: number; durMs: number;
  onStart: (ms: number) => void; onEnd: (ms: number) => void;
}) {
  return (
    <div className="e-field2">
      <label className="e-field"><span className="e-fl">Start</span>
        <NumberField min={0} max={secOf(endMs)} value={secOf(startMs)} onChange={(v) => onStart(Math.round(v * 1000))} /></label>
      <label className="e-field"><span className="e-fl">End</span>
        <NumberField min={secOf(startMs)} max={secOf(durMs)} value={secOf(endMs)} onChange={(v) => onEnd(Math.round(v * 1000))} /></label>
    </div>
  );
}

export interface SegOption { key: string; label: string; on: boolean; disabled?: boolean; title?: string; visual?: ReactNode }

export function SegRow({ options, onPick, thumbs = false, ariaLabel }: {
  options: SegOption[]; onPick: (key: string) => void; thumbs?: boolean; ariaLabel: string;
}) {
  const still = useReducedMotion();
  // Per-instance, so the gliding tick never flies between two rows - not between a layout
  // segment's entry and exit curve rows, and not between the outgoing and incoming inspectors
  // during PropertiesSlot's swap, when both are briefly mounted.
  const id = useId();
  return (
    <div className={`e-preseg${thumbs ? " thumbs" : ""}`} role="group" aria-label={ariaLabel}>
      {options.map((o) => (
        <motion.button key={o.key} type="button" className={`e-preseg-b${o.on ? " on" : ""}`}
          aria-pressed={o.on} disabled={o.disabled} title={o.title ?? o.label}
          whileTap={o.disabled || still ? undefined : PRESS} transition={PRESS_SPRING}
          onClick={() => onPick(o.key)}>
          {o.visual}
          <span className="e-preseg-t">{o.label}</span>
          {o.on && (still
            ? <span className="e-preseg-tick" />
            : <motion.span layoutId={`seg-${id}`} className="e-preseg-tick" transition={GLIDE} />)}
        </motion.button>
      ))}
    </div>
  );
}

export function RemoveButton({ label, onClick }: { label: string; onClick: () => void }) {
  const still = useReducedMotion();
  return (
    <motion.button type="button" className="e-del" title={label} onClick={onClick}
      whileTap={still ? undefined : PRESS} transition={PRESS_SPRING}>
      <IconTrash size={15} />{label}
    </motion.button>
  );
}
