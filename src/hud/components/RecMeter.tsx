import { VoiceWave } from "../../lib/wave/ui/VoiceWave";
import { Mic, MicOff } from "./icons";

/** Width and height of the meter's drawing area, px. `useHudWindowSize.ts`'s `RECMETER_W` term
 *  carries this width, so the recording window stays sized to what the meter actually needs. */
export const METER_W = 104;
export const METER_H = 30;

/** The recording bar's mic indicator: the icon, and the live voice wave beside it.
 *
 *  State honesty (task-6 (c)/(i), user-reported): the meter renders ONLY when audio is actually
 *  being captured - when muted an explicit "Muted" chip replaces it instead of a fake or frozen
 *  wave, and when the mic is on but no level report has arrived (permission pending, no device, a
 *  driver reset mid-take) `live` is false and `VoiceWave` drops to the line colour rather than
 *  drawing a resting wave that would read as a working microphone. The levels themselves come
 *  from the Rust capture that is writing the WAV (`useAudioLevels`), so there is no path here that
 *  can show a level for audio this take is not recording.
 *
 *  The meter's own geometry is not this file's business: it hands `VoiceWave` a box and a level
 *  getter, and `voiceWave.ts` decides what a frame looks like inside it. */
export function RecMeter({ micOn, live, read }: {
  micOn: boolean; live: boolean; read: () => { mic: number; sys: number };
}) {
  return (
    <div className={`recmeter ${micOn ? "" : "muted"}`}>
      <span className="ico">{micOn ? <Mic /> : <MicOff />}</span>
      {micOn
        ? <VoiceWave w={METER_W} h={METER_H} read={read} live={live} />
        : <span className="muted-label">Muted</span>}
    </div>
  );
}
