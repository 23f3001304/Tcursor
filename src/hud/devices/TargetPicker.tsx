import { useEffect, useRef } from "react";
import { Chevron, Check, Monitor } from "../components/icons";
import { parseTarget, type DisplayInfo } from "./selectDevices";

/** Capture-target picker: a `Dropdown`-alike for `listDisplays()` results, but decorates each
 *  row with its resolution and a "Primary" badge for the main display, and groups `kind:
 *  "window"` targets under their own header - purely presentational, the target_id values and
 *  selection wiring are unchanged from the plain dropdown this replaces. */
export function TargetPicker({ targets, value, open, onToggle, onPick }: {
  targets: DisplayInfo[];
  value: string;
  open: boolean;
  onToggle: () => void;
  onPick: (id: string) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onToggle();
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [open, onToggle]);

  const rows = targets.map((t, i) => ({ t, meta: parseTarget(t, i) }));
  const screens = rows.filter((r) => r.t.kind !== "window");
  const windows = rows.filter((r) => r.t.kind === "window");
  const selected = rows.find((r) => r.t.id === value);

  const row = (r: (typeof rows)[number]) => (
    <button key={r.t.id} className={`dd-item tp-item ${r.t.id === value ? "sel" : ""}`} onClick={() => onPick(r.t.id)}>
      <span className="tp-main">
        <span className="dd-item-label">{r.meta.title}</span>
        {r.meta.resolution && <span className="tp-res">{r.meta.resolution}</span>}
      </span>
      <span className="tp-trail">
        {r.meta.primary && <span className="tp-badge">Primary</span>}
        {r.t.id === value && <span className="dd-check"><Check /></span>}
      </span>
    </button>
  );

  return (
    <div className={`dd ${open ? "open" : ""}`} ref={ref}>
      <button className="dd-trigger" onClick={onToggle}>
        <span className="ico"><Monitor /></span>
        <span className="dd-label">{selected?.meta.title ?? "—"}</span>
        <span className="chev"><Chevron /></span>
      </button>
      {open && (
        <div className="dd-menu tp-menu">
          {screens.map(row)}
          {windows.length > 0 && <div className="tp-divider">Windows</div>}
          {windows.map(row)}
        </div>
      )}
    </div>
  );
}
