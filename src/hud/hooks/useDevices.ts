import { useEffect, useState } from "react";
import { listDisplays, listAudioInputs } from "../../lib/ipc";
import { pickDefaults, type DeviceState, type DisplayInfo, type AudioInfo } from "../devices/selectDevices";

export function useDevices() {
  const [displays, setDisplays] = useState<DisplayInfo[]>([]);
  const [mics, setMics] = useState<AudioInfo[]>([]);
  const [sel, setSel] = useState<DeviceState>({ displayId: null, micId: null });
  useEffect(() => {
    (async () => {
      const [d, m] = await Promise.all([listDisplays(), listAudioInputs()]);
      setDisplays(d); setMics(m); setSel(pickDefaults(d, m));
    })();
  }, []);
  return { displays, mics, sel, setSel };
}
