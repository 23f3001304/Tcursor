import { memo } from "react";
import type { Aspect, EditDoc, EditOp } from "../../../shared/edit";
import type { ClickSample } from "../../../shared/ipc";
import type { Range } from "../../timeline/useRangeSelect";
import { TransportTools } from "./TransportTools";
import { PlaybackGroup } from "./PlaybackGroup";
import { OutputGroup } from "./OutputGroup";

export const Transport = memo(function Transport({
  timeMs,
  dur,
  outTimeMs,
  outDur,
  plain,
  playing,
  onPlay,
  onSeek,
  onAddZoom,
  onAutoedit,
  aiRunning,
  exporting,
  trimmed,
  onTrimIn,
  onTrimOut,
  onResetTrim,
  aspect,
  onAspect,
  quality,
  onQuality,
  muted,
  onMute,
  volume,
  onVolume,
  clicks,
  range,
  setRange,
  onApply,
  onDetectSilences,
}: {
  timeMs: number;
  dur: number;
  outTimeMs: number;
  outDur: number;
  plain: boolean;
  playing: boolean;
  onPlay: () => void;
  onSeek: (ms: number) => void;
  onAddZoom: () => void;
  onAutoedit: () => void;
  aiRunning: boolean;
  exporting: boolean;
  trimmed: boolean;
  onTrimIn: () => void;
  onTrimOut: () => void;
  onResetTrim: () => void;
  aspect: Aspect;
  onAspect: (aspect: Aspect) => void;
  quality: number;
  onQuality: () => void;
  muted: boolean;
  onMute: () => void;
  volume: number;
  onVolume: (v: number) => void;
  clicks: ClickSample[];
  range: Range | null;
  setRange: (r: Range | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onDetectSilences: () => void;
}) {
  const locked = exporting || dur <= 0;

  return (
    <div className="e-transport">
      <TransportTools
        locked={locked}
        trimmed={trimmed}
        onTrimIn={onTrimIn}
        onTrimOut={onTrimOut}
        onResetTrim={onResetTrim}
        onAddZoom={onAddZoom}
        onAutoedit={onAutoedit}
        aiRunning={aiRunning}
        exporting={exporting}
        timeMs={timeMs}
        dur={dur}
        clicks={clicks}
        range={range}
        setRange={setRange}
        onApply={onApply}
        onDetectSilences={onDetectSilences}
      />
      <div className="e-tdiv" />

      <PlaybackGroup
        timeMs={timeMs}
        dur={dur}
        outTimeMs={outTimeMs}
        outDur={outDur}
        plain={plain}
        playing={playing}
        locked={locked}
        onPlay={onPlay}
        onSeek={onSeek}
      />
      <div className="e-tdiv" />

      <OutputGroup
        locked={locked}
        aspect={aspect}
        onAspect={onAspect}
        quality={quality}
        onQuality={onQuality}
        muted={muted}
        onMute={onMute}
        volume={volume}
        onVolume={onVolume}
      />
    </div>
  );
});
