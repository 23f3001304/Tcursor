# src/editor/director/DirectorPointer.tsx

The AI director's fake cursor: a violet-ringed (`--e-ai`) arrow that visibly performs each edit instead of applying it invisibly. The CALLER mounts/unmounts it (`{running && <DirectorPointer .../>}` inside an `AnimatePresence` - see `DirectorOverlay.tsx`) so it fades in/out with the run itself.

## DirectorPointerHandle

```ts
export interface DirectorPointerHandle {
  moveTo(x: number, y: number): Promise<void>;
  press(): Promise<void>;
  sweep(fromX: number, toX: number, y: number): Promise<void>;
}
```

The imperative surface `useDirector.ts`'s `reveal` loop drives.

- `moveTo(x, y)` - glides to `(x, y)` via the spring `x`/`y` motion values (`.set`, stiffness 170 / damping 26). Resolves once both are within 2px of the target (a `MotionValue.on("change", ...)` listener), or after `pace(distance).travelCapMs` - whichever comes first, so a long-distance move (or one that overshoots and never quite settles) can never stall the reveal.
- `press()` - a ~180ms "click": sets `pressed` true for 90ms (the ring pulses 18px/20% alpha -> 10px/60% alpha, the glyph dips to 0.92 scale) then false for another 90ms (both animate back), resolving after both halves.
- `sweep(fromX, toX, y)` - jumps instantly (`.jump`, no spring) to `(fromX, y)`, then runs a plain 320ms tween (`animate(x, toX, {type: "tween", ...})`, NOT spring-driven) across to `toX` - used for `clear_zooms`, which has no single point to aim at.

## DirectorPointer

```tsx
export function DirectorPointer({ ref }: { ref?: Ref<DirectorPointerHandle> }): JSX.Element
```

Renders the fake cursor and exposes `DirectorPointerHandle` on `ref` (React 19 ref-as-prop - no `forwardRef` wrapper).

### Behavior

**Mount-time snap.** A mount-only effect reads `anchorPoint("wand")` (`targets.ts`) and `.jump`s `x`/`y` straight there (no glide) - so when the caller's `AnimatePresence` fades the element in right after, it reads as "appearing at the button", not flying in from the origin `(0, 0)`.

**Rendering.** A `motion.div.e-director-ptr` positioned via `style={{x, y}}` (the spring motion values), `initial={{opacity:0}}`/`animate={{opacity:1}}`/`exit={{opacity:0}}` (0.16s) for the mount/unmount fade. Inside it: `.e-director-ring` (a `motion.span`, animates `width`/`height`/`opacity` on `pressed`) and `.e-director-glyph` (a `motion.svg` arrow, `--e-fg` fill with a 1.5px `--e-bg` outline, animates `scale` on `pressed`).

### Notes

- `x`/`y` are stable `useSpring` instances for the component's lifetime (React re-mounts a fresh pair each time the caller mounts `DirectorPointer`, since it's conditionally rendered).
- No `AbortController` - every handle method is a small, self-contained `Promise`; cancellation is handled one level up, by `useDirector.ts`'s `cancelRef` simply not starting the next step.
