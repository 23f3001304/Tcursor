export interface DisplayInfo { id: string; label: string; kind?: string }
export interface AudioInfo { id: string; label: string }
export interface DeviceState { displayId: string | null; micId: string | null }

export function pickDefaults(displays: DisplayInfo[], mics: AudioInfo[]): DeviceState {
  return {
    displayId: displays[0]?.id ?? null,
    micId: mics[0]?.id ?? null,
  };
}

/** Split a capture target's backend-formatted label into a clean title + optional resolution.
 *  `list_displays` bakes "(Primary)" or "(WxH)" onto the end of a display's label - pulling it
 *  out lets the picker render it as a distinct badge/sub-line instead of raw parenthetical text.
 *  `primary` is true only for index 0 of the raw `listDisplays()` result: the backend always
 *  enumerates the main monitor first (before any other displays or windows). */
export function parseTarget(t: DisplayInfo, index: number): { title: string; resolution: string | null; primary: boolean } {
  const m = /^(.*) \((Primary|\d+x\d+)\)$/.exec(t.label);
  const title = m ? m[1] : t.label;
  const resolution = m && m[2] !== "Primary" ? m[2] : null;
  return { title, resolution, primary: index === 0 && t.kind !== "window" };
}
