export interface DisplayInfo {
  id: string;
  label: string;
  kind?: string;
}
export interface AudioInfo {
  id: string;
  label: string;
}
export interface DeviceState {
  displayId: string | null;
  micId: string | null;
}

export function pickDefaults(displays: DisplayInfo[], mics: AudioInfo[]): DeviceState {
  return {
    displayId: displays[0]?.id ?? null,
    micId: mics[0]?.id ?? null,
  };
}

export function resolveSelection(prev: DeviceState, displays: DisplayInfo[], mics: AudioInfo[]): DeviceState {
  return {
    displayId: displays.some((d) => d.id === prev.displayId) ? prev.displayId : (displays[0]?.id ?? null),
    micId: mics.some((m) => m.id === prev.micId) ? prev.micId : (mics[0]?.id ?? null),
  };
}

export function parseTarget(
  t: DisplayInfo,
  index: number,
): { title: string; resolution: string | null; primary: boolean } {
  if (t.kind === "window") return { title: prettifyWindowLabel(t.label), resolution: null, primary: false };
  const m = /^(.*) \((?:(\d+x\d+)(, Primary)?|(Primary))\)$/.exec(t.label);
  const title = m ? m[1] : t.label;
  return { title, resolution: m?.[2] ?? null, primary: Boolean(m?.[3] || m?.[4]) || index === 0 };
}

export function cleanDeviceLabel(raw: string): string {
  let s = raw.trim();
  const wrapped =
    /^(?:Microphone(?:\s+Array)?|Headset(?:\s+(?:Microphone|Earphone))?|Headphones?|Speakers?|Line In)\s*\((.+)\)$/i.exec(
      s,
    );
  if (wrapped) s = wrapped[1];
  s = s.replace(/^\d+-\s*/, "");
  s = s.replace(/\s*\(Windows Virtual Camera\)$/i, "");
  return s.trim();
}

export function isOwnProcessWindow(t: DisplayInfo): boolean {
  return t.kind === "window" && /(?:^|[\\/])tcursor-scaffold\.exe$/i.test(t.label);
}

export function prettifyWindowLabel(label: string): string {
  const m = /^(App: )([a-zA-Z]:\\.+|\\\\.+)$/.exec(label);
  if (!m) return label;
  const base = m[2].split(/[\\/]/).pop() ?? m[2];
  return m[1] + base.replace(/\.[a-zA-Z0-9]{1,6}$/, "");
}
