import { motion } from "motion/react";
import { Camera, CameraOff, Cpu, Mic, MicOff, Speaker, SpeakerOff } from "./icons";

const TOGGLE_PRESS = {
  whileTap: { scale: 0.96 },
  transition: { type: "spring" as const, stiffness: 500, damping: 30 },
};

export interface Toggles {
  camOn: boolean;
  micOn: boolean;
  sysOn: boolean;
  gameMode: boolean;
}

export function SourceToggles({
  toggles: t,
  onToggle,
}: {
  toggles: Toggles;
  onToggle: (key: keyof Toggles) => void;
}) {
  return (
    <div className="tog-row" role="group" aria-label="Sources">
      <motion.button
        className={`tog ${t.camOn ? "on" : ""}`}
        aria-pressed={t.camOn}
        aria-label="Camera"
        title={t.camOn ? "Camera on" : "Camera off"}
        onClick={() => onToggle("camOn")}
        {...TOGGLE_PRESS}
      >
        {t.camOn ? <Camera /> : <CameraOff />}
      </motion.button>
      <motion.button
        className={`tog ${t.micOn ? "on" : ""}`}
        aria-pressed={t.micOn}
        aria-label="Microphone"
        title={t.micOn ? "Microphone on" : "Microphone off"}
        onClick={() => onToggle("micOn")}
        {...TOGGLE_PRESS}
      >
        {t.micOn ? <Mic /> : <MicOff />}
      </motion.button>
      <motion.button
        className={`tog ${t.sysOn ? "on" : ""}`}
        aria-pressed={t.sysOn}
        aria-label="System audio"
        title={t.sysOn ? "System audio on" : "System audio off"}
        onClick={() => onToggle("sysOn")}
        {...TOGGLE_PRESS}
      >
        {t.sysOn ? <Speaker /> : <SpeakerOff />}
      </motion.button>
      <motion.button
        className={`tog ${t.gameMode ? "on" : ""}`}
        aria-pressed={t.gameMode}
        aria-label="Compatibility encoder"
        title={
          t.gameMode
            ? "Compatibility encoder on: the legacy CPU path"
            : "Compatibility encoder: only if GPU recording has issues"
        }
        onClick={() => onToggle("gameMode")}
        {...TOGGLE_PRESS}
      >
        <Cpu />
      </motion.button>
    </div>
  );
}
