import { memo } from "react";
import { IconVolume, IconMicrophone } from "@tabler/icons-react";
import { Shimmer } from "./Shimmer";

export const AudioTrack = memo(function AudioTrack({
  src,
  kind,
  loading,
}: {
  src: string;
  kind: "system" | "mic";
  loading: boolean;
}) {
  if (!src) return loading ? <Shimmer className="e-audiorow" /> : null;
  const Icon = kind === "mic" ? IconMicrophone : IconVolume;
  return (
    <div className="e-audiorow">
      <span className="e-trackicon" title={kind === "mic" ? "Microphone" : "System audio"}>
        <Icon size={13} />
      </span>
      <img src={src} alt="" draggable={false} />
    </div>
  );
});
