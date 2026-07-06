export interface DisplayInfo { id: number; label: string }
export interface AudioInfo { id: string; label: string }
export interface DeviceState { displayId: number | null; micId: string | null }

export function pickDefaults(displays: DisplayInfo[], mics: AudioInfo[]): DeviceState {
  return {
    displayId: displays[0]?.id ?? null,
    micId: mics[0]?.id ?? null,
  };
}
