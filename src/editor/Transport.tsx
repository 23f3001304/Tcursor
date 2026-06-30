import { IconZoomIn, IconBulb, IconPlayerSkipBack, IconPlayerPlay, IconPlayerPause, IconPlayerSkipForward, IconVolume, IconVolumeOff, IconArrowsMaximize } from "@tabler/icons-react";
import { fmt } from "./time";

/** Transport bar under the stage: add-zoom/spotlight tools, play controls, the time readout,
 *  the preview-quality toggle, and the audio mute toggle. */
export function Transport({ timeMs, dur, playing, onPlay, onSeek, onAddZoom, onAddSpotlight, quality, onQuality, muted, onMute }: {
  timeMs: number; dur: number; playing: boolean; onPlay: () => void; onSeek: (ms: number) => void;
  onAddZoom: () => void; onAddSpotlight: () => void; quality: number; onQuality: () => void; muted: boolean; onMute: () => void;
}) {
  return (
    <div className="e-transport">
      <button className="e-tg" title="Add zoom at playhead" onClick={onAddZoom}><IconZoomIn size={17} /></button>
      <button className="e-tg" title="Add spotlight at playhead" onClick={onAddSpotlight}><IconBulb size={16} /></button>
      <div className="e-sp" />
      <button className="e-tg" title="Jump to start" onClick={() => onSeek(0)}><IconPlayerSkipBack size={16} /></button>
      <button className="e-play" title={playing ? "Pause" : "Play"} onClick={onPlay}>
        {playing ? <IconPlayerPause size={16} /> : <IconPlayerPlay size={16} />}
      </button>
      <button className="e-tg" title="Jump to end" onClick={() => onSeek(dur)}><IconPlayerSkipForward size={16} /></button>
      <span className="e-time">{fmt(timeMs)} / {fmt(dur)}</span>
      <div className="e-sp" />
      <button className="e-qual" title="Preview quality (proxy resolution)" onClick={onQuality}>{quality}p</button>
      <button className={`e-tg${muted ? " on" : ""}`} title={muted ? "Unmute" : "Mute"} onClick={onMute}>
        {muted ? <IconVolumeOff size={16} /> : <IconVolume size={16} />}
      </button>
      <button className="e-tg" title="Fit to window"><IconArrowsMaximize size={16} /></button>
    </div>
  );
}
