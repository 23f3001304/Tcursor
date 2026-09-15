let on = true;
const subs = new Set<(v: boolean) => void>();

export function interfaceEffectsOn(): boolean {
  return on;
}

export function setInterfaceEffects(v: boolean): void {
  if (v === on) return;
  on = v;
  for (const fn of [...subs]) fn(v);
}

export function onInterfaceEffectsChange(fn: (v: boolean) => void): () => void {
  subs.add(fn);
  return () => {
    subs.delete(fn);
  };
}
