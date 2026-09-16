# src/editor/panels/background/GradeSection.tsx

The Color section of the Background panel (spec 3.6): a Look picker over the nine grade presets plus exposure, contrast and vignette to taste. Its own file rather than more JSX in `BackgroundPanel.tsx`, matching how `WallpaperTab` and `GradientTab` are already split out, and because that panel sits close to its 200-line budget.

It renders as the LAST child of the panel, AFTER the closing `</Disclosure>`, not inside it. The disclosure holds tuning applied to a background you have already chosen; a grade is a choice of its own about the whole picture, so it stays in sight.

## NO_GRADE

```ts
export const NO_GRADE: GradeSettings = { preset: "none", exposure: 0, contrast: 1, vignette: 0 }
```

The identity grade, and the value the panel's Reset writes. Mirrors Rust's `GradeSettings::default()`; a document carrying it renders byte for byte as one from before the grade existed, because `params_of` returns `None` for it.

It lives here rather than beside `DEFAULT_BG` in `backgroundPresets.ts` because it is this section's own contract: the picker's `none` row and this constant have to agree, and they are three lines apart.

## GradeSection

```tsx
export function GradeSection({ grade, onChange }: {
  grade: GradeSettings;
  onChange: (next: GradeSettings) => void;
}): JSX.Element
```

Four controls over `settings.grade`.

| control | kind | range | step | format |
|---|---|---|---|---|
| Look | `Picker` | the nine presets | | None, Cinematic, Noir, Vintage, Frost, Golden, Midnight, Vivid, Dreamy |
| Exposure | `Slider` | -2 to +2 stops | 0.05 | `+0.35 EV`, always signed |
| Contrast | `Slider` | 0.5 to 1.8 | 0.02 | `112%` |
| Vignette | `Slider` | 0 to 100 | 1 | `28%` |

Vignette is the one control whose displayed number is not the stored one: the setting is 0 to 1 and the slider is 0 to 100, converted at both ends, because a percentage is what the user is thinking in and a fraction is what the formula takes.

**`onChange` always carries the WHOLE next `GradeSettings`,** never a patch. Picking a look writes all four fields in one `onSaveSettings` call, so choosing Cinematic is ONE undo step rather than four, and the preview repaints once rather than four times. `pick` seeds the three stored numbers from `seedOf(preset)`; `set` spreads the current grade and overrides one slider.

**Why picking a look WRITES the numbers instead of just recording the name.** That is M3's "presets are starting points" contract (spec 3.1): the three stored numbers are ABSOLUTE, so the user can pick Noir and then pull the vignette back without the document needing a fourth state between "a preset" and "custom". The picker shows the look that was picked until the numbers drift from its row, at which point it reads **Custom** - see `hasDrifted`. The document is unchanged by that: `preset` still says `noir`, because a bent Noir is still a Noir the user started from, and picking Noir again re-seeds all three numbers from its row.

**A `Picker`, not a grid of thumbnails.** The owner's standing ruling is plain dropdowns over card editors (`classic-layout-kept-shell-vetoed`), and nine one-word looks is exactly what a dropdown is for. Rendering nine live-graded thumbnails would also mean nine WebGL passes per open.

The hint line says the look applies to the whole picture and that captions, text and the cursor keep their own colours, which is the paint order of spec 1.3 stated once where a user can see it: the grade covers the background, screen and webcam, and stops before anything whose colour they chose.

### Used by

- `src/editor/panels/background/BackgroundPanel.tsx` - the only consumer, and the source of both the value and the save

### Behaviors

- `offers the nine looks and writes all four grade fields in one save` - opening the Look picker lists the nine labels in table order, and choosing Cinematic produces exactly one save carrying `{ preset: "cinematic", exposure: 0, contrast: 1.12, vignette: 0.28 }`, which is spec 3.2's Cinematic row.
- `puts the grade back to none when the panel is reset` - the panel's Reset writes `NO_GRADE` over a Noir.
- `reads Custom once a knob drifts off the picked look, and the look again when re-picked` (`GradeSection.test.tsx`) - pick Noir and the three numbers are Noir's row and the button reads Noir; one ArrowRight on Exposure and it reads Custom while `preset` is still `noir`; pick Noir again and all three numbers are back to the row and the button reads Noir.
- `never reads Custom on the None look, whose knobs are the user's own` - bending exposure over None leaves the button reading None.

## hasDrifted

```ts
export function hasDrifted(g: GradeSettings): boolean
```

Whether the three stored numbers have been bent away from their preset's row, which is what makes the Look picker read **Custom** instead of the preset's name. Compared against `seedOf(g.preset)` with an epsilon of `1e-6`, because the sliders commit stepped floats (`0.05`, `0.02`, `0.01`) and an exact comparison would call a round trip back to the seeded value drift.

`preset: "none"` is never drifted. None has no look to drift FROM: its row is the identity, and a user who bends exposure on it is tuning by hand rather than departing from something they picked. Reading "Custom" there would be true but useless, and it would hide the fact that no look is applied.

**This is UI-only, by controller ruling.** Nothing is written to the document, nothing crosses the wire, and neither `GradeSettings` nor `GradeParams` gains a field. The state is derived on every render from two values the component already has, which is also what makes it self-correcting: drag a knob back to the seeded number and the picker reads the look's name again with no bookkeeping.

### Used by

- `src/editor/panels/background/GradeSection.tsx` - the `label` override handed to the Look `Picker`
