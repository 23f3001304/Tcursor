# src/editor/stage/transport/PlaybackGroup.tsx

The transport bar's centre cluster: skip-to-start, the Play/Pause hero, skip-to-end and the time readout. Split out of `Transport.tsx` - it is the one group with behaviour of its own (the magnetic pull needs a ref and a hook), and it is the group that re-renders every frame during playback because the readout is live.

## PlaybackGroup

```tsx
export function PlaybackGroup(props: {
  timeMs: number; dur: number; outTimeMs: number; outDur: number; plain: boolean;
  playing: boolean; locked: boolean; onPlay: () => void; onSeek: (ms: number) => void;
}): JSX.Element
```

All props come straight from `Transport`, which documents where each one is owned; `locked` (`exporting || dur <= 0`) is computed there and passed in so the three clusters cannot disagree about it.

**Playback controls (design/premium-pass D3: Play restyled as the bar's hero).** Skip-to-start (title "Jump to the start") calls `onSeek(0)`; skip-to-start/end are now `motion.button`s carrying the app-wide press spring (design/premium-pass D6: `whileTap: { scale: 0.96 }`, unconditional - neither is gated by `locked`) since they previously had no press feedback at all. Play/Pause (`.e-play`, `disabled={locked}`) calls `onPlay()` and shows `IconPlayerPause`/`IconPlayerPlay` by `playing` (both `fill="currentColor"` so they render as solid glyphs rather than outlines; the play triangle carries NO nudge - Tabler draws it right of its own box centre already, and the old `marginLeft: 2` on top of that put it about 3px right of the disc's centre, which the owner saw as misaligned on 2026-09-14). Play gets its own spring (`PLAY_SPRING`: `type: "spring", stiffness: 500, damping: 30`, the same spring `TopBar`'s Export button now uses) rather than `TAP_SPRING`, with `whileTap: { scale: 0.94 }` (omitted while `locked`) - snappier than the ghost buttons around it. A `.e-play-glow` overlay (the existing `--e-glow-primary` soft-glow token) fires a one-shot 150ms fade-out pulse on every play/pause toggle: its `key` alternates between `"on"`/`"off"` with `playing`, so React remounts it (replaying `initial={{opacity:1}} -> animate={{opacity:0}}`) each time `playing` flips, with no separate pulse-tracking state needed. Skip-to-end (title "Jump to the end") calls `onSeek(dur)`.

**Magnetic pull on Play (micro-interaction pass, 2026-09-14).** Play leans toward a pointer that comes within 28px of it, by a quarter of the pointer's offset from its centre, on a softer spring (300/20) than the press above. It is the beat *before* the hover: the button acknowledges the approach, so arriving on it feels like the pointer was caught rather than merely landed. The Trim In/Out pills beside it do the same, so the three lean together as the pointer crosses the bar - see [useMagnetic](../effects/useMagnetic.md) and [TransportTools](TransportTools.md).

The hook's two spring `MotionValue`s go **straight onto the `motion.button`'s `style`** as `x`/`y`, with no wrapper element: `PLAY_TAP` is `{ scale: 0.94 }`, so the press never touches the translate channel and the two cannot fight over one transform. `locked` passes `strength: 0`, which makes the hook a complete no-op (no listener, no spring) - the same rule the press spring already follows one paragraph up: a button that will ignore the click does not invite one. With `interface_effects` off in Preferences, or `prefers-reduced-motion` set, the hook adds nothing and Play renders exactly as it did before this pass.

**Time readout.** The current position renders via `fmtPrecise` (`src/editor/timeline/model/time.ts`, "M:SS.s" - one decisecond) in `.e-time`'s own 13px/600 `--e-fg`; the dimmed `.tot` total still uses plain `fmt` ("M:SS", no decimal) so only the live position reads to a tenth of a second.
