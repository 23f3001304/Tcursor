import { useState } from "react";
import type { HotkeySettings } from "./settings";

const ROWS: { key: keyof HotkeySettings; label: string }[] = [
  { key: "zoom_hold", label: "Hold to zoom" },
  { key: "spotlight_hold", label: "Spotlight hold" },
  { key: "video_fx_hold", label: "Video effect hold" },
  { key: "layout_screen", label: "Layout: Screen" },
  { key: "layout_camera", label: "Layout: Camera" },
  { key: "layout_presenter", label: "Layout: Presenter" },
  { key: "layout_screen_only", label: "Layout: Screen only" },
  { key: "layout_camera_only", label: "Layout: Camera only" },
];

/** Build a chord string ("Ctrl+Alt+Z") from a keydown, or null if only modifiers
 *  are held. Mirrors the Rust KeyChord format (Ctrl/Alt/Shift + one A-Z/0-9 key). */
export function chordFromEvent(e: KeyboardEvent): string | null {
  const k = e.key.toUpperCase();
  if (k === "CONTROL" || k === "ALT" || k === "SHIFT" || k === "META") return null;
  if (!/^[A-Z0-9]$/.test(k)) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  parts.push(k);
  return parts.join("+");
}

/** The chord strings that are bound to more than one action. */
export function conflicts(h: HotkeySettings): Set<string> {
  const seen = new Map<string, number>();
  for (const { key } of ROWS) { const c = h[key]; seen.set(c, (seen.get(c) ?? 0) + 1); }
  return new Set([...seen].filter(([, n]) => n > 1).map(([c]) => c));
}

export function SettingsHotkeys({ value, onChange }: { value: HotkeySettings; onChange: (v: HotkeySettings) => void }) {
  const [capturing, setCapturing] = useState<keyof HotkeySettings | null>(null);
  const dup = conflicts(value);

  function capture(key: keyof HotkeySettings, e: React.KeyboardEvent) {
    e.preventDefault();
    const chord = chordFromEvent(e.nativeEvent);
    if (chord) { onChange({ ...value, [key]: chord }); setCapturing(null); }
    else if (e.key === "Escape") setCapturing(null);
  }

  return (
    <section className="sec">
      <h3 className="sec-title">Hotkeys</h3>
      {ROWS.map(({ key, label }) => (
        <div className="sf-row hk" key={key}>
          <span className="sf-label">{label}</span>
          <button
            className={`hk-chord ${capturing === key ? "cap" : ""} ${dup.has(value[key]) ? "dup" : ""}`}
            onClick={() => setCapturing(key)}
            onKeyDown={(e) => capturing === key && capture(key, e)}
          >
            {capturing === key ? "press keys…" : value[key]}
          </button>
        </div>
      ))}
      {dup.size > 0 && <div className="hk-warn">Two actions share a shortcut.</div>}
    </section>
  );
}
