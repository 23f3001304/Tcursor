# src/editor/inspectors/SpringControls.tsx

The two parameters that shape a spring, rendered under `MotionField`'s graph. A spring is **not** a cubic bezier, so the canvas' two drag handles cannot express one - dragging them would silently convert the value to a cubic and throw the physics away, which is why the canvas hides them for a spring and shows these instead. The canvas still draws the real sampled oscillator (`springPathOf`), so the sliders and the picture agree.

## SpringControls

```tsx
export function SpringControls({ stiffness, damping, mass, onChange }: {
  stiffness: number; damping: number; mass: number; onChange: (easing: string) => void;
}): JSX.Element
```

### Props

- `stiffness, damping, mass: number` - the current oscillator, as `MotionField` resolved it from the wire-name via `springOf` (so the bare word `"spring"` arrives already expanded to `SPRING_DEFAULT`).
- `onChange: (easing: string) => void` - the same commit callback `MotionField` was given (it writes the spring onto both ramps). Every slider move emits a full canonical `formatSpring(...)` string, so the first drag on a bare `"spring"` promotes it to `spring(100.000,10.000,1.000)` and it stays parameterised from then on.

### What the two sliders actually do

Only the **damping ratio** `c / (2*sqrt(k*m))` changes the drawn curve. The export remaps a spring's response onto its own settle time (`export/spring.md`), so `w0` cancels: raising stiffness and damping together is a visual no-op, while raising either one alone is not. Both sliders are therefore meaningful - it is only that one particular combination which does nothing, and a user moving one slider at a time never hits it.

Bounds come from `SPRING_RANGE` (`shared/math/spring.md`), which mirrors the Rust `STIFFNESS`/`DAMPING` consts - so the UI cannot author a value `parse_spring` would clamp, and a slider's extremes are the wire's extremes.

**Mass is on the wire but not exposed.** It is redundant with stiffness for shaping (both enter only through `zeta` and the cancelled `w0`), so a third slider would be a second way to do the same thing. `SpringControls` passes the current `mass` through untouched, so a value authored elsewhere survives a slider drag.

### Styling

An `.e-field2` pair of `.e-field` labels wrapping the shared `Slider` (`controls/fields/Slider.tsx`), which brings its own `.e-fl` readout, debounced commit and WAI-ARIA slider keyboard handling - the same control the rest of the inspectors use, so a spring parameter behaves exactly like Scale or Zoom in.
