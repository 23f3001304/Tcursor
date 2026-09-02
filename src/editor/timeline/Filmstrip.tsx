import { memo } from "react";
import { Shimmer } from "./Shimmer";

/** The clip track: a strip of evenly-spaced frame thumbnails (from ensure_thumbs) filling the
 *  width, so the timeline shows the recording's frames like Filmora. Non-interactive (seeks
 *  pass through to the track); a quiet shimmer skeleton (same size as the loaded strip) shows
 *  until the thumbnails load - a real recording always eventually produces at least one frame,
 *  so an empty `thumbs` here is always "still loading", never a legitimate final state.
 *  `React.memo`'d - `thumbs` only changes once per project load, so this never needs to
 *  re-render for a playhead tick or an unrelated edit. */
export const Filmstrip = memo(function Filmstrip({ thumbs }: { thumbs: string[] }) {
  if (!thumbs.length) return <Shimmer className="e-filmstrip" />;
  return (
    <div className="e-filmstrip">
      {thumbs.map((src, i) => (
        <img key={i} src={src} alt="" draggable={false} style={{ width: `${100 / thumbs.length}%` }} />
      ))}
    </div>
  );
});
