export const TAP_SPRING = {
  whileHover: { scale: 1.04 },
  whileTap: { scale: 0.98 },
  transition: { type: "tween" as const, duration: 0.12, ease: [0.4, 0, 0.2, 1] as const },
};
export const PLAY_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };
export const PLAY_TAP = { scale: 0.94 };
export const PRESS_TAP = { scale: 0.96 };
