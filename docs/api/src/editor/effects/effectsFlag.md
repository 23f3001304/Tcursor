# src/editor/effects/effectsFlag.ts

The one on/off switch for the editor's interface micro-interactions (`InterfaceSettings.interface_effects`), as a three-function module store.

**Why a module store and not a context.** The feature has two halves that cannot see each other. [InterfaceEffects](InterfaceEffects.md) is mounted from `src/App.tsx` beside `<Editor>` and is the only thing that ever calls `getSettings()`; [useMagnetic](useMagnetic.md) runs inside `PlayButton` and `TransportTools`, three levels down a tree neither of them owns. A React context would need a provider wrapped around `Editor` - and `Editor.tsx` / `shell/ClassicShell.tsx` belong to another agent's surface. A module-level `let` needs no provider at all, and it is readable synchronously from a plain event handler, which is what keeps `useMagnetic`'s rAF path free of React.

**Why it starts `true`.** The settings read is async. Starting `false` would flash the effects off on the first frame for everyone who has them on, which is the common case. Someone who has turned them **off** loses at most one pointerdown's worth of ripple at editor open, before the `getSettings()` promise lands.

**Scope.** Module-global for the process, which is correct: there is one editor per window and one settings file behind it.

## interfaceEffectsOn

```ts
export function interfaceEffectsOn(): boolean
```

The current value. Synchronous, no hook, no provider - safe to call from an event handler, a rAF callback, or a `useState` initializer (which is how both consumers seed themselves).

## setInterfaceEffects

```ts
export function setInterfaceEffects(v: boolean): void
```

Publish a new value and notify every subscriber. Called only by `InterfaceEffects`, from its `getSettings()` resolution.

**No-op when unchanged.** The settings are re-read on every window focus, and the overwhelmingly common answer is "the same as last time". Bailing out here means a focus event cannot churn subscribers or re-render the transport when nothing has actually moved.

Subscribers are notified from a copy of the set (`[...subs]`), so a subscriber that unsubscribes inside its own callback cannot mutate the set mid-iteration.

## onInterfaceEffectsChange

```ts
export function onInterfaceEffectsChange(fn: (v: boolean) => void): () => void
```

Subscribe to changes. Returns the unsubscribe function, so an effect can `return` it directly:

```ts
useEffect(() => onInterfaceEffectsChange(setOn), []);
```

The callback receives the new value; it is never called with the value it was registered at, only on an actual change.
