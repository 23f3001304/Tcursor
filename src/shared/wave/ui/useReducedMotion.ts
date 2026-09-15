import { useEffect, useState } from "react";

export function useReducedMotion(): boolean {
  const supported = typeof matchMedia === "function";
  const [reduced, setReduced] = useState(
    () => supported && matchMedia("(prefers-reduced-motion: reduce)").matches,
  );
  useEffect(() => {
    if (!supported) return;
    const mq = matchMedia("(prefers-reduced-motion: reduce)");
    const onChange = () => setReduced(mq.matches);
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [supported]);
  return reduced;
}
