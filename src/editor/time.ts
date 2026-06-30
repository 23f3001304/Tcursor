/** "M:SS" from milliseconds (transport clock). */
export function fmt(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

export interface Tick { at: number; label: string }

/** Adaptive ruler ticks at "nice" intervals so labels never repeat (a 2s clip gets
 *  0.5s ticks, a 5min clip gets 30s ticks). Each tick carries its time position (ms)
 *  and a label whose precision matches the interval. */
export function rulerTicks(dur: number): Tick[] {
  if (dur <= 0) return [{ at: 0, label: "0:00" }];
  const steps = [200, 500, 1000, 2000, 5000, 10000, 15000, 30000, 60000, 120000, 300000, 600000];
  const step = steps.find((s) => s >= dur / 6) ?? steps[steps.length - 1];
  const dec = step < 1000 ? 1 : 0; // sub-second steps need a decimal to stay distinct
  const ticks: Tick[] = [];
  for (let t = 0; t <= dur + 1; t += step) ticks.push({ at: t, label: tlabel(t, dec) });
  return ticks;
}

function tlabel(ms: number, dec: number): string {
  const total = Math.max(0, ms) / 1000;
  const m = Math.floor(total / 60);
  const s = total - m * 60;
  const ss = dec > 0 ? s.toFixed(dec).padStart(dec + 3, "0") : String(Math.round(s)).padStart(2, "0");
  return `${m}:${ss}`;
}
