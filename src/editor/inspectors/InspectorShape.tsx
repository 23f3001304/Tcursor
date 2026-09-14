import { useId, type ReactNode } from "react";
import { motion, useReducedMotion } from "motion/react";
import { IconChevronDown, IconChevronUp, IconTrash, IconX } from "@tabler/icons-react";
import { NumberField } from "../controls/Controls";

const PRESS = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };
const GLIDE = { type: "spring" as const, stiffness: 500, damping: 40 };
/** One nudge of a `ValueRow` cell, in clip seconds - the step every inspector field already uses. */
const CELL_STEP = 0.05;

export const secOf = (ms: number) => +(ms / 1000).toFixed(2);

export const secText = (ms: number) => `${(Math.max(0, ms) / 1000).toFixed(2)}s`;

export const spanRange = (startMs: number, endMs: number) => `${secText(startMs)} to ${secText(endMs)}`;

export type InspectorKind = "zoom" | "fx" | "layout" | "cam" | "cut" | "speed";

export function InspectorShell({ kind, children }: { kind: InspectorKind; children: ReactNode }) {
  return <div className={`e-panel e-insp e-insp-${kind}`}>{children}</div>;
}

export function InspectorHeader({ title, range, deleteLabel, onDelete, onClose, thumb }: {
  title: string; range: string; deleteLabel: string;
  onDelete: () => void; onClose: () => void; thumb?: ReactNode;
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
        <motion.button type="button" className="e-ihicon del" title={deleteLabel} aria-label={deleteLabel}
          onClick={onDelete} whileTap={tap} transition={PRESS_SPRING}><IconTrash size={15} /></motion.button>
        <motion.button type="button" className="e-ihicon" title="Deselect" aria-label="Deselect"
          onClick={onClose} whileTap={tap} transition={PRESS_SPRING}><IconX size={15} /></motion.button>
      </div>
      <p className="e-ihead-range" title={range}>{range}</p>
    </header>
  );
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

export interface ValueCell {
  label: string; sec: number; min: number; max: number; onChange: (sec: number) => void;
}

/** How many columns `cells` are laid out in: one each up to three, then 2 x 2. Four across a 360px
 *  sidebar leaves a cell 82px, and a cell needs 20 of padding, 16 of steppers and room for a
 *  reading like "125.30s" - so `ZoomInspector`'s Start/End/In/Out row was both cutting its own
 *  digits off against `.e-ivals`' `overflow: hidden` AND spilling its fourth cell onto a second
 *  row of a three-column grid, where it drew a divider against nothing. */
export const valueRowColumns = (count: number) => (count > 3 ? 2 : Math.max(1, count));

export function ValueRow({ cells, ariaLabel }: { cells: ValueCell[]; ariaLabel: string }) {
  const cols = valueRowColumns(cells.length);
  return (
    <div className="e-ivals" role="group" aria-label={ariaLabel}
      style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
      {cells.map((c, i) => {
        const set = (raw: number) => c.onChange(Math.min(c.max, Math.max(c.min, +raw.toFixed(2))));
        return (
          // The hairlines are per-cell rather than an `+` rule, so they follow a wrapped row: a
          // left edge on every cell that does not start a row, a top edge on every cell below the
          // first row. `--e-divider` is the sheet's quietest hairline, the same one `.e-isec` uses.
          <div className="e-ival" key={c.label} style={{ boxShadow: [
            i % cols > 0 ? "inset 1px 0 0 var(--e-divider)" : "",
            i >= cols ? "inset 0 1px 0 var(--e-divider)" : "",
          ].filter(Boolean).join(", ") || undefined }}>
            <span className="e-ival-l">{c.label}</span>
            <div className="e-ival-row">
              <span className="e-ival-v">{c.sec.toFixed(2)}<i>s</i></span>
              <span className="e-ival-steps">
                <button type="button" title={`More ${c.label}`} aria-label={`More ${c.label}`}
                  disabled={c.sec >= c.max} onClick={() => set(c.sec + CELL_STEP)}><IconChevronUp size={11} /></button>
                <button type="button" title={`Less ${c.label}`} aria-label={`Less ${c.label}`}
                  disabled={c.sec <= c.min} onClick={() => set(c.sec - CELL_STEP)}><IconChevronDown size={11} /></button>
              </span>
            </div>
          </div>
        );
      })}
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
