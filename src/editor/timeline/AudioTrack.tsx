import { IconVolume, IconMicrophone } from "@tabler/icons-react";
import { Shimmer } from "./Shimmer";

/** One audio track row: the source's waveform image (from ensure_waveform) as a quiet,
 *  low-contrast background, with a small left icon to tell system vs mic apart. While `loading`
 *  (the waveform fetch hasn't resolved yet - `useEditorData`'s `wavesReady`), shows a shimmer
 *  skeleton instead; once resolved, an empty `src` means this project genuinely has no audio for
 *  this source (e.g. no mic was recorded) and the row renders nothing. Non-interactive - seeks
 *  pass through. */
export function AudioTrack({ src, kind, loading }: { src: string; kind: "system" | "mic"; loading: boolean }) {
  if (!src) return loading ? <Shimmer className="e-audiorow" /> : null;
  const Icon = kind === "mic" ? IconMicrophone : IconVolume;
  return (
    <div className="e-audiorow">
      <span className="e-trackicon" title={kind === "mic" ? "Microphone" : "System audio"}><Icon size={13} /></span>
      <img src={src} alt="" draggable={false} />
    </div>
  );
}
