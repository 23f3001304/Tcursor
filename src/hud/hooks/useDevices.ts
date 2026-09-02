import { useEffect, useState } from "react";
import { listDisplays, listAudioInputs } from "../../lib/ipc";
import { resolveSelection, type DeviceState, type DisplayInfo, type AudioInfo } from "../devices/selectDevices";

export function useDevices() {
  const [displays, setDisplays] = useState<DisplayInfo[]>([]);
  const [mics, setMics] = useState<AudioInfo[]>([]);
  const [sel, setSel] = useState<DeviceState>({ displayId: null, micId: null });

  useEffect(() => {
    let cancelled = false;
    const refresh = async () => {
      const [d, m] = await Promise.all([listDisplays(), listAudioInputs()]);
      if (cancelled) return;
      setDisplays(d); setMics(m);
      setSel((prev) => resolveSelection(prev, d, m));
    };
    refresh();
    // Plugging/unplugging a display or mic doesn't otherwise re-run this - without it, a picked
    // device that just disappeared stays selected until Record is pressed and Rust's cpal open
    // fails (surfaced today via `record-warning`, but only at that point). Re-enumerating here
    // catches it earlier and swaps in a still-present device instead.
    navigator.mediaDevices?.addEventListener("devicechange", refresh);
    return () => { cancelled = true; navigator.mediaDevices?.removeEventListener("devicechange", refresh); };
  }, []);

  return { displays, mics, sel, setSel };
}
