/** The clip track: a strip of evenly-spaced frame thumbnails (from ensure_thumbs) filling the
 *  width, so the timeline shows the recording's frames like Filmora. Non-interactive (seeks
 *  pass through to the track); an empty well shows until the thumbnails load. */
export function Filmstrip({ thumbs }: { thumbs: string[] }) {
  if (!thumbs.length) return (
    <div className="e-filmstrip e-filmstrip-empty">Generating thumbnails…</div>
  );
  return (
    <div className="e-filmstrip">
      {thumbs.map((src, i) => (
        <img key={i} src={src} alt="" draggable={false} style={{ width: `${100 / thumbs.length}%` }} />
      ))}
    </div>
  );
}
