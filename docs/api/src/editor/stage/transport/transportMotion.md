# src/editor/stage/transport/transportMotion.ts

The transport's one motion language, shared by `Transport.tsx` and `TransportTools.tsx` so the two files cannot drift.

## TAP_SPRING

```ts
export const TAP_SPRING = { whileHover: { scale: 1.04 }, whileTap: { scale: 0.98 }, transition: { type: "tween", duration: 0.12, ease: [0.4, 0, 0.2, 1] } };
```

The tool buttons' hover-lift and press, a tween.

## PLAY_SPRING

```ts
export const PLAY_SPRING = { type: "spring", stiffness: 500, damping: 30 };
```

The press spring for Play, the skip buttons, mute and the trim reset.

## PLAY_TAP

```ts
export const PLAY_TAP = { scale: 0.94 };
```

Play is the hero and presses deeper.

## PRESS_TAP

```ts
export const PRESS_TAP = { scale: 0.96 };
```
