/** The transport's one motion language, shared by `Transport` and `TransportTools`: a press spring
 *  (500/30) with a .94 hero press for Play and .96 for everything else, and a tween hover-lift for
 *  the tool buttons. Defined once so the two files cannot drift. */
export const TAP_SPRING = { whileHover: { scale: 1.04 }, whileTap: { scale: 0.98 }, transition: { type: "tween" as const, duration: 0.12, ease: [0.4, 0, 0.2, 1] as const } };
export const PLAY_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };
export const PLAY_TAP = { scale: 0.94 };
export const PRESS_TAP = { scale: 0.96 };
