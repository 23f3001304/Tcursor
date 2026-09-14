export interface DisplayInfo { id: string; label: string; kind?: string }
export interface AudioInfo { id: string; label: string }
export interface DeviceState { displayId: string | null; micId: string | null }

export function pickDefaults(displays: DisplayInfo[], mics: AudioInfo[]): DeviceState {
  return {
    displayId: displays[0]?.id ?? null,
    micId: mics[0]?.id ?? null,
  };
}

/** Re-resolve a device selection against a fresh enumeration: keep the current pick if it is
 *  still present in the new lists, otherwise fall back to the first available device (or `null`
 *  if none). Used by `useDevices`'s `devicechange` handler so a plug/unplug doesn't silently keep
 *  pointing at a device that is gone - the root cause of a `start_recording` call opening a stale
 *  mic id and getting nothing (surfaced today via `record-warning`, but only AFTER Record is
 *  pressed) - while also not blowing away a deliberate pick that is still there. */
export function resolveSelection(prev: DeviceState, displays: DisplayInfo[], mics: AudioInfo[]): DeviceState {
  return {
    displayId: displays.some((d) => d.id === prev.displayId) ? prev.displayId : (displays[0]?.id ?? null),
    micId: mics.some((m) => m.id === prev.micId) ? prev.micId : (mics[0]?.id ?? null),
  };
}

/** Split a capture target's backend-formatted label into a clean title + optional resolution.
 *  `list_displays` bakes "(WxH, Primary)", "(Primary)" or "(WxH)" onto the end of a display's
 *  label - pulling it out lets the card and the sheet render it as a badge, a sub-line and a
 *  to-scale rectangle instead of raw parenthetical text. `primary` is true for the label that
 *  says so, and for index 0 of the raw `listDisplays()` result regardless: the backend always
 *  enumerates the main monitor first (before any other displays or windows). Windows never carry
 *  a resolution/primary suffix - their raw title just gets `prettifyWindowLabel`'s path cleanup. */
export function parseTarget(t: DisplayInfo, index: number): { title: string; resolution: string | null; primary: boolean } {
  if (t.kind === "window") return { title: prettifyWindowLabel(t.label), resolution: null, primary: false };
  const m = /^(.*) \((?:(\d+x\d+)(, Primary)?|(Primary))\)$/.exec(t.label);
  const title = m ? m[1] : t.label;
  return { title, resolution: m?.[2] ?? null, primary: Boolean(m?.[3] || m?.[4]) || index === 0 };
}

/** Strip Windows/cpal's device-name packaging for display. Only the *label* shown in a dropdown
 *  changes - `sel.micId`/`camId` keep the raw string, which still round-trips into
 *  `startRecording`/`getUserMedia` unchanged.
 *
 *  Two independent shapes get unwrapped, in this order:
 *  1. A generic Windows audio category wrapping the real product name in parens, e.g.
 *     `"Microphone (3- Insta360 Link 2C)"`, `"Headset (WH-1000XM4 Hands-Free AG Audio)"`,
 *     `"Microphone Array (Realtek High Definition Audio)"` (a laptop's built-in array mic), or
 *     `"Headset Microphone (2- Realtek(R) Audio)"` (a Bluetooth/USB headset's mic endpoint) - the
 *     whitelist covers both the one-word and two-word category forms Windows actually reports.
 *     The match is greedy to the *last* `)`, so a nested-paren product name like
 *     `"Microphone (Realtek(R) Audio)"` survives intact.
 *  2. cpal/Windows' own numeric enumeration prefix inside that name, e.g. `"3- Insta360 Link 2C"`.
 *
 *  Decision (documented, not inferred): the `"(Windows Virtual Camera)"` suffix some webcams
 *  report via their MediaFoundation frame-server shim is dropped too - it names a driver
 *  implementation detail, not the physical device, and the same hardware's cpal *mic* label never
 *  carries an analogous suffix, so keeping it would make one physical device read as two
 *  unrelated ones across the mic and camera dropdowns. */
export function cleanDeviceLabel(raw: string): string {
  let s = raw.trim();
  const wrapped = /^(?:Microphone(?:\s+Array)?|Headset(?:\s+(?:Microphone|Earphone))?|Headphones?|Speakers?|Line In)\s*\((.+)\)$/i.exec(s);
  if (wrapped) s = wrapped[1];
  s = s.replace(/^\d+-\s*/, "");
  s = s.replace(/\s*\(Windows Virtual Camera\)$/i, "");
  return s.trim();
}

/** `list_displays` (commands.rs) filters the exact string `"TCursor"` out of the window list -
 *  the HUD's own titled window. It does not catch a second, distinct case: before a window's
 *  title is set (or for a window Windows never gave one), `EnumWindows` can report the raw
 *  executable path as the title instead, e.g. `"App: C:\Users\...\tcursor-scaffold.exe"` for this
 *  very process. Rust is out of scope for this pass, so filter that shape here as a second line
 *  of defense - the app should never list itself as a capture target. */
export function isOwnProcessWindow(t: DisplayInfo): boolean {
  return t.kind === "window" && /(?:^|[\\/])tcursor-scaffold\.exe$/i.test(t.label);
}

/** A window title that is itself a filesystem path (the same raw-path shape `isOwnProcessWindow`
 *  looks for, but for *any* process, not just this one) reads far better as its filename with no
 *  extension than as the full path. */
export function prettifyWindowLabel(label: string): string {
  const m = /^(App: )([a-zA-Z]:\\.+|\\\\.+)$/.exec(label);
  if (!m) return label;
  const base = m[2].split(/[\\/]/).pop() ?? m[2];
  return m[1] + base.replace(/\.[a-zA-Z0-9]{1,6}$/, "");
}
