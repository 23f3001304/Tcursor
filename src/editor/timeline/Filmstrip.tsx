import { Shimmer } from "./Shimmer";

/** The clip track: a strip of evenly-spaced frame thumbnails (from ensure_thumbs) filling the
 *  width, so the timeline shows the recording's frames like Filmora. Non-interactive (seeks
 *  pass through to the track); a quiet shimmer skeleton (same size as the loaded strip) shows
 *  until the thumbnails load - a real recording always eventually produces at least one frame,
 *  so an empty `thumbs` here is always "still loading", never a legitimate final state. */
export function Filmstrip({ thumbs }: { thumbs: string[] }) {
  if (!thumbs.length) return <Shimmer className="e-filmstrip" />;
  return (
    <div className="e-filmstrip">
      {thumbs.map((src, i) => (
        <img key={i} src={src} alt="" draggable={false} style={{ width: `${100 / thumbs.length}%` }} />
      ))}
    </div>
  );
}
