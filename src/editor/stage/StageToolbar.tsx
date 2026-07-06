import { useState } from "react";
import { IconAspectRatio, IconClick, IconTypography, IconVideo } from "@tabler/icons-react";

const TOOLS = [
  { id: "aspect", icon: IconAspectRatio, label: "Aspect ratio" },
  { id: "cursor", icon: IconClick, label: "Cursor" },
  { id: "captions", icon: IconTypography, label: "Captions" },
  { id: "camera", icon: IconVideo, label: "3D camera" },
];

/** Floating stage overlay: quick toggles for the preview. Each button carries a real .on
 *  active state so the row reads as controls, not dead chrome. */
export function StageToolbar() {
  const [on, setOn] = useState<Record<string, boolean>>({});
  const toggle = (id: string) => setOn((s) => ({ ...s, [id]: !s[id] }));
  return (
    <div className="e-ftool">
      {TOOLS.map(({ id, icon: Icon, label }) => (
        <button key={id} className={on[id] ? "on" : ""} title={label} aria-label={label}
          aria-pressed={!!on[id]} onClick={() => toggle(id)}><Icon size={18} /></button>
      ))}
    </div>
  );
}
