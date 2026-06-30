import { IconVolume, IconMicrophone } from "@tabler/icons-react";

/** One audio track row: the source's waveform image (from ensure_waveform) as a quiet,
 *  low-contrast background, with a small left icon to tell system vs mic apart. Hidden when
 *  that source wasn't recorded (empty src). Non-interactive - seeks pass through. */
export function AudioTrack({ src, kind }: { src: string; kind: "system" | "mic" }) {
  if (!src) return null;
  const Icon = kind === "mic" ? IconMicrophone : IconVolume;
  return (
    <div className="e-audiorow">
      <span className="e-trackicon" title={kind === "mic" ? "Microphone" : "System audio"}><Icon size={13} /></span>
      <img src={src} alt="" draggable={false} />
    </div>
  );
}
