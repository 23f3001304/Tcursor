import { useMemo, useSyncExternalStore, type CSSProperties } from "react";
import { densityFor, densityVars, sameDensity, type Density } from "./density";

const fallback: Density = densityFor(1440, 900);
let current: Density = fallback;
const subs = new Set<() => void>();
let bound = false;

function measure(): Density {
  if (typeof window === "undefined") return fallback;
  return densityFor(window.innerWidth || 0, window.innerHeight || 0);
}

function onResize(): void {
  const next = measure();
  if (sameDensity(next, current)) return;
  current = next;
  for (const fn of subs) fn();
}

function subscribe(fn: () => void): () => void {
  subs.add(fn);
  if (!bound && typeof window !== "undefined") {
    window.addEventListener("resize", onResize);
    bound = true;
  }
  return () => {
    subs.delete(fn);
  };
}

function snapshot(): Density {
  const next = measure();
  if (!sameDensity(next, current)) current = next;
  return current;
}

export function useDensity(): Density {
  return useSyncExternalStore(subscribe, snapshot, () => current);
}

export function useDensityVars(): CSSProperties {
  const d = useDensity();
  return useMemo(() => densityVars(d), [d]);
}
