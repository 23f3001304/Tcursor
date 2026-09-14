import { AppWindow, Back, Check, Monitor } from "../components/icons";
import { isOwnProcessWindow, parseTarget, type DisplayInfo } from "./selectDevices";

/** The "what to record" sheet the idle card flips to from its Screen row (2026-09-14: a dropdown
 *  list of displays and windows had nowhere to open inside a vertical card). One list in two
 *  sections: every display as a row (monitor glyph, its name, its size and a Primary tag on one
 *  sub-line, a check on the chosen one), then every window the same way with a window glyph. A
 *  map of the monitors was tried first and vetoed on sight (owner: "just make it a list"). One
 *  tap picks and returns (`onPick`); the back arrow returns without picking (`onBack`). */
export function TargetSheet({ targets, value, onPick, onBack }: {
  targets: DisplayInfo[]; value: string; onPick: (id: string) => void; onBack: () => void;
}) {
  // `index` runs over the RAW list: parseTarget's index-0 primary rule is keyed off it.
  const rows = targets.map((t, i) => ({ t, meta: parseTarget(t, i) })).filter((r) => !isOwnProcessWindow(r.t));
  const screens = rows.filter((r) => r.t.kind !== "window");
  const windows = rows.filter((r) => r.t.kind === "window");
  const row = (r: (typeof rows)[number], label: string, sub: string | null, glyph: React.ReactNode) => (
    <button key={r.t.id} type="button" role="option" aria-selected={r.t.id === value}
      className={`dd-item tgt${r.t.id === value ? " sel" : ""}`} onClick={() => onPick(r.t.id)}>
      <span className="ico">{glyph}</span>
      <span className="dd-main">
        <span className="dd-item-label">{label}</span>
        {sub && <span className="dd-sub">{sub}</span>}
      </span>
      {r.t.id === value && <span className="dd-check"><Check /></span>}
    </button>
  );
  return (
    <div className="sheet">
      <div className="sheet-head">
        <button type="button" className="winbtn" title="Back" aria-label="Back" onClick={onBack}><Back /></button>
        <span className="sheet-title">What to record</span>
      </div>
      <div className="sheet-list" role="listbox" aria-label="What to record">
        <div className="sheet-sec">Displays</div>
        {screens.map((r) => row(r, r.meta.title.replace(/^Display \d+: /, ""),
          [r.meta.resolution?.replace("x", " by "), r.meta.primary ? "Primary" : null].filter(Boolean).join(" \u00b7 ") || null, <Monitor />))}
        {windows.length > 0 && <div className="sheet-sec">Windows</div>}
        {windows.map((r) => row(r, r.meta.title.replace(/^App: /, ""), null, <AppWindow />))}
      </div>
    </div>
  );
}
