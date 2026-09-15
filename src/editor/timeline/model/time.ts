export function fmt(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

export function fmtPrecise(ms: number): string {
  return tlabel(ms, 1);
}

export interface Tick {
  at: number;
  label: string;
}

export function rulerTicks(dur: number): Tick[] {
  if (dur <= 0) return [{ at: 0, label: "0:00" }];
  const steps = [200, 500, 1000, 2000, 5000, 10000, 15000, 30000, 60000, 120000, 300000, 600000];
  const step = steps.find((s) => s >= dur / 6) ?? steps[steps.length - 1];
  const dec = step < 1000 ? 1 : 0;
  const ticks: Tick[] = [];
  for (let t = 0; t <= dur + 1; t += step) ticks.push({ at: t, label: tlabel(t, dec) });
  return ticks;
}

function tlabel(ms: number, dec: number): string {
  const total = Math.max(0, ms) / 1000;
  const m0 = Math.floor(total / 60);
  const s0 = total - m0 * 60;
  const rounded = dec > 0 ? Number(s0.toFixed(dec)) : Math.round(s0);
  const carry = rounded >= 60;
  const m = carry ? m0 + 1 : m0;
  const s = carry ? 0 : rounded;
  const ss = dec > 0 ? s.toFixed(dec).padStart(dec + 3, "0") : String(s).padStart(2, "0");
  return `${m}:${ss}`;
}
