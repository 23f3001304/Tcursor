import { useEffect, useRef } from "react";
import type { Caption } from "../../../shared/edit";
import { captionAt } from "../../stage/fx/captionPreview";
import { secText } from "../../inspectors/InspectorShape";

export function CaptionList({
  captions,
  timeMs,
  sel,
  onPick,
}: {
  captions: Caption[];
  timeMs: number;
  sel: string | null;
  onPick: (c: Caption) => void;
}) {
  const live = captionAt(captions, timeMs);
  const liveId = live?.id ?? null;
  const liveRow = useRef<HTMLButtonElement | null>(null);

  // INVARIANT: follow the playhead only when the LIVE caption changes, never on every tick -
  // a scroll on each frame would fight the hand that is scrolling the list.
  useEffect(() => {
    if (liveId) liveRow.current?.scrollIntoView({ block: "nearest" });
  }, [liveId]);

  return (
    <ul className="e-caplist">
      {captions.map((c) => (
        <li key={c.id}>
          <button
            type="button"
            ref={c.id === liveId ? liveRow : null}
            className={`e-caplist-row${c.id === liveId ? " live" : ""}${c.id === sel ? " on" : ""}`}
            aria-current={c.id === liveId ? "true" : undefined}
            aria-pressed={c.id === sel}
            onClick={() => onPick(c)}
          >
            <span className="e-captime">{secText(c.start_ms)}</span>
            <span className="e-captxt">{c.text}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}
