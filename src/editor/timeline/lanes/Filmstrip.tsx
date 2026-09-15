import { memo } from "react";
import { Shimmer } from "./Shimmer";

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
