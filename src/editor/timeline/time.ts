/** "M:SS" from milliseconds (transport clock). */
export function fmt(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** "M:SS.s" from milliseconds - `fmt`'s one-decisecond sibling (design/premium-pass D3's transport
 *  readout wants the CURRENT time precise to a tenth of a second; the dimmed total stays `fmt`'s
 *  plain "M:SS"). Reuses `tlabel`, the same decimal formatter `rulerTicks` already uses for its
 *  sub-second ruler ticks, rather than a second parallel implementation. */
export function fmtPrecise(ms: number): string {
  return tlabel(ms, 1);
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

/* Fix round 1 (design/premium-pass D3 review): rounding `s` to `dec` decimals AFTER splitting off
   whole minutes could round a value like 59.96s up to "60.0" while `m` had already been floored
   from the un-rounded total, producing an impossible "0:60.0" readout at every minute boundary
   during live playback (`fmtPrecise` is the first caller to feed tlabel a continuous, un-quantized
   `ms` - rulerTicks' inputs are always exact multiples of its own tick step, so they never actually
   land on a value that rounds up past :60). Fixed by rounding the SECONDS remainder to display
   precision first, then carrying into `m` if that rounds all the way up to 60 - `s` is provably
   always < 60 before rounding (it's `total` modulo 60), so it can only ever round UP TO exactly 60,
   never past it, meaning `m += 1, s = 0` is the only carry case that can occur. */
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
