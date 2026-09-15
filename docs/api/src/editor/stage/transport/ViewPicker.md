# src/editor/stage/transport/ViewPicker.tsx

The transport's view-mode control: Fit / Fill / 100%, in the right group beside the quality chip.

## ViewPicker

```tsx
export function ViewPicker(): JSX.Element
```

Three buttons in one `.e-viewseg` track (the app's segmented idiom, one plane lower to suit the raised bar it sits in: an inset well at the canvas plane with the chosen option as a raised pill on it, at the 28px the chips beside it stand at), rendered from `VIEW_MODES` so the order and the labels live in one place with the geometry that implements them.

### No props

The choice is session UI state in `viewMode.ts`, read here with `useViewMode` and written with `setViewMode`; `Stage` reads the same store. See `viewMode.md` for why it is a store rather than state lifted into `ClassicShell`.

### Behavior

Each button carries `aria-pressed` and the mode's own sentence as its `title` ("Fit the whole frame inside the stage", "Fill the stage, cropping the frame's long edge", "Source pixels, centred - never upscaled"), and takes the transport's shared press spring (`PRESS_TAP`/`PLAY_SPRING` from `transportMotion.ts`) so it presses like every other control in the bar. The group itself is labelled "Preview view mode" for screen readers.

### Why it lives in the transport

The owner's brief: nothing floats over the preview any more. A view control is a property of how the preview is shown, so it belongs with the other two view chips (aspect, quality) rather than in a layer over the video.
