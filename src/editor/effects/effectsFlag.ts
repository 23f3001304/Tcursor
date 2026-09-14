/** The one on/off switch for the editor's interface micro-interactions, shared by the two halves
 *  of the feature that cannot see each other: `InterfaceEffects` (the ripple overlay, which OWNS
 *  the value - it is the only thing that reads `getSettings()`) and `useMagnetic` (mounted inside
 *  `PlayButton` and `TransportTools`, three levels down a tree neither of them controls).
 *
 *  A module-level `let` plus a subscriber set rather than a React context, deliberately: a context
 *  would need a provider wrapped around `Editor`, and `Editor.tsx`/`ClassicShell.tsx` belong to
 *  another agent. It also means `interfaceEffectsOn()` is readable from a plain event handler with
 *  no hook at all, which is what keeps the magnetic hook's rAF path free of React.
 *
 *  It starts `true` so the first frame after mount already has the effects on - the settings read
 *  is async, and starting `false` would flash the feature off for anyone who has it on (the common
 *  case). Someone who has turned it OFF loses one pointerdown's worth of ripple at editor open. */
let on = true;
const subs = new Set<(v: boolean) => void>();

/** The current value, readable synchronously from anywhere (no hook, no provider). */
export function interfaceEffectsOn(): boolean { return on; }

/** Publish a new value. A no-op when unchanged, so a `getSettings()` re-read on every window
 *  focus cannot churn subscribers (or re-render the transport) when nothing has actually moved. */
export function setInterfaceEffects(v: boolean): void {
  if (v === on) return;
  on = v;
  for (const fn of [...subs]) fn(v);
}

/** Subscribe to changes; returns the unsubscribe, so an effect can `return` it directly. */
export function onInterfaceEffectsChange(fn: (v: boolean) => void): () => void {
  subs.add(fn);
  return () => { subs.delete(fn); };
}
