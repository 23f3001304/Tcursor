# src/editor/stage/transport/OutputGroup.tsx

The transport bar's right cluster: the aspect chip, the preview-quality chip, the view-mode control and mute plus its volume flyout - everything about how the preview is being *shown* rather than how it is playing. Split out of `Transport.tsx`, taking the aspect cycle table and the bar's only piece of local state (the flyout's open/closed flag) with it.

## OutputGroup

```tsx
export function OutputGroup(props: {
  locked: boolean; aspect: Aspect; onAspect: (aspect: Aspect) => void;
  quality: number; onQuality: () => void;
  muted: boolean; onMute: () => void; volume: number; onVolume: (v: number) => void;
}): JSX.Element
```

All props come straight from `Transport`, which documents where each one is owned. `ASPECT_ORDER`/`ASPECT_LABEL` are module-private here: there is one aspect control in the app and this is it.

**Aspect chip.** `cycleAspect` finds `aspect`'s index in `ASPECT_ORDER` and calls `onAspect` with the next one (wrapping); the chip label comes from `ASPECT_LABEL[aspect]` (`"Source"`, `"16:9"`, `"9:16"`, `"1:1"`, `"4:3"`). Both constants are module-private again as of the look pass: the floating stage toolbar that carried the editor's second copy of this cycle is gone, so nothing outside this file imports them and there is only one aspect control left to keep in step.

**View mode (look pass, 2026-09-14).** `<ViewPicker />` renders straight after the quality chip: Fit / Fill / 100%, with no props - it and `Stage` share the choice through `viewMode.ts`, which is where the geometry and the reasoning live. It is the one control this pass added to the bar, and it is in the bar rather than over the preview because nothing floats over the preview any more.

**Mute button.** Shows `IconVolumeOff` when `muted` is true OR `volume === 0` (so a slider dragged all the way down reads as muted even before the mute button itself is clicked), otherwise `IconVolume`. The click handler is always `onMute` - a plain toggle that doesn't touch `volume`. Also a `motion.button` with the app-wide press spring (design/premium-pass D6), unconditional since mute is never locked.

**Volume flyout.** Hovering `.e-volwrap` reveals (`AnimatePresence` fade+slide, no width animation) a `Slider` bound directly to the `volume`/`onVolume` props (`0..100`, displaying `0` while `muted` without altering the underlying `volume` value) - a real, `Editor`-owned control now rather than a local mock. Dragging it above `0` while `muted` also calls `onMute()`, so raising the slider from a muted state unmutes rather than silently changing a number nobody hears. **Hover bridge (Task 36):** `.e-volwrap` has no flex `gap` between the mute button and `.e-volflyout` - the same 6px of breathing room instead lives as `.e-volflyout`'s own padding, so the button and flyout sit flush with zero dead space between them and the wrap's hover state (which shows/hides the flyout) never has a gap to lose the pointer over while crossing from the button onto the slider.

**It opens to the LEFT, over the bar (width audit, 2026-09-14).** Volume is the last control in the transport and the flyout is 76px, so anchored at the wrapper's right edge it always reached past the bar's own right edge - and since the bar is centred at up to `100% - 32px`, past the *window* on anything under about 1150px, where `.editor`'s `overflow: hidden` simply ate it. Opening inward can never leave the window. It gains the raised plane and the panel radius so it reads as a slot that slid out of the button rather than a transparent strip laid over the view-mode control it covers, and the Motion `x` flips sign (`+6 -> 0`) so it still slides out of the button.
