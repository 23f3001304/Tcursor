# src/editor/panels/PanelHeader.tsx

The shared header every rail panel renders (the inspectors have their own `InspectorHeader`, `inspectors/InspectorShape.tsx`): a title, an optional one-line lede, an optional thumb, and the matched ghost icon buttons (reset, close). It is also where `panels/panels.css` enters the bundle, the same way `Controls.tsx` carries `controls.css` - every panel renders this, so importing the stylesheet here means no panel can be styled but unstyled.

## PanelHeader

```tsx
export function PanelHeader({ title, lede, thumb, onReset, onClose, closeTitle }: {
  title: string; lede?: string; thumb?: React.ReactNode;
  onReset?: () => void; onClose: () => void; closeTitle?: string;
}): JSX.Element
```

### Props

- `thumb?` (T34 L4) - a small visual beside the title (`LayoutInspector`'s 48x28 arrangement schematic). It joins the SAME flex group as the `<h2>`, not a third child of `.e-phead-top`, so `space-between` still only ever balances two things: that group against `.e-hicons`. Omitted - which is what the other twelve-plus call sites do - a lone `<h2>` reads identically to the old bare markup.
- `onReset?` / `onClose` / `closeTitle?` - the two ghost icon buttons. Both carry a `title` AND an `aria-label`: an icon-only control with neither is the benchmark's tell 5.

### Look (panel pass, 2026-09-13)

**No rule under the header.** The hairline is gone; the gap to the first group is the separation (20px in the panel pass, 16 since the usability pass tightened the section rhythm). This is the "quiet chrome, no bordered boxes" rule applied to the one piece of chrome every panel shows.

**The lede is one line at hint size**, ellipsised, with the whole sentence as the element's `title`. Panel ledes were rewritten short enough to fit; an inspector's measured lede (`CutInspector`'s "removes 1.5 s") can still run past 320px, and truncating it is better than letting a two-line description push every control down. The text is unchanged in the DOM, so callers' tests that read `.e-lede` as text are unaffected.

`.e-phead*` and `.e-hicons` moved from `editor.css` to `panels/panels.css`. `.e-hicon` itself stayed behind: the settings dialog's sections use it too, and this pass does not touch that surface.

### panels.css

The panels' stylesheet, imported here. Six rules run through it:

1. **Surfaces read by lightness, not strokes.** `--e-raised` at rest, `--e-raised-hi` on hover, `--e-bg` for a groove or a well. No panel element carries a 1px border any more.
2. **One accent** (`--e-primary`), only for on/selected state: the Switch's engaged track, a selected tile's 2px inset ring, a chosen dropdown option's 3px tick. Colour swatches are the documented exception (`Swatches.md`).
3. **One motion language:** press spring 500/30 at scale 0.96, hover lift `y: -1` (rows and pills, not tiles), content swaps as a 0.16s `[0.4, 0, 0.2, 1]` tween, and `useReducedMotion` honoured wherever a layout spring or glide exists.
4. **At most ONE level of card in a panel** - `.e-tile` (`PackTile.md`), which since the arrangements pass is every picture tile in the editor: a cursor pack, a wallpaper and a gradient preset are all that one card. Sections, rows and switch-plus-dependents group by SPACING, via `.e-grp` (16px between groups, 10px between rows inside one, 6px from a row's label to its control). A `.e-grp.e-secstack` closes to 2px, for a run of `CategorySection` headers that read as one library rather than one section each.
5. **Nothing hugs the left.** Every full-width control spans the content box, so a panel has no dead column on the right at any width.
6. **A panel fits its slot without scrolling** - about 620px on a 720px-tall window (usability pass, 2026-09-13). See the budget below.

### The height budget (usability pass, 2026-09-13)

The owner's verdict after the panel pass was that the panels were "still not good from a usability point of view" and that Background "needs a lot of scrolling". It was: about 1939px of content in a 620px slot. Every panel was then measured by rows at 320px wide, padding included at both ends, and brought under 620 with four devices, in order of how much they buy:

- **Collapsed `CategorySection`s** for preset libraries (`CategorySection.md`): the wallpapers, the gradient presets and the cursor packs. This entry read "`TileRow` strips" until the arrangements pass (2026-09-14) replaced them - the strips bought the same height by scrolling sideways, which the owner rejected outright; a stack of closed section headers buys slightly more (one 28px header per group, plus the one open section) and hides nothing behind a flick.
- **`.e-two` two-up rows** for pairs of short controls whose own inline readouts already name them.
- **A 16px section rhythm** in place of 20, and a 20px bottom pad in place of 24. Small, but it is five to seven gaps per panel.
- **One `Disclosure` per panel** (`Disclosure.md`), last, holding that panel's least-used controls - and only where the first three were not enough.

| panel | before | after | fits 620 | More holds |
|---|---|---|---|---|
| Background (Wallpapers) | 1939 | 576 | yes | Look, Frame, Accent Colors |
| Background (Gradient) | 970 | 458 | yes | as above |
| Background (Color) | 685 | 364 | yes | as above |
| Cursor (Enhanced) | 957 | 450 | yes | Motion, Click |
| Camera (Rounded, ring on) | 764 | 542 | yes | the ring, Margin X and Margin Y |
| Effects (clicks on) | 1064 | 558 | yes | Spotlight, Video effect |
| AI Director | 345 | 333 | yes | nothing - it fits |
| Audio | 327 | 342 | yes | nothing - it fits |
| Captions | 171 | 163 | yes | nothing - it fits |

Audio is the one panel that grew: it is three sliders, and the 24px hit-target floor (`Controls.md` rule 4) costs 9px a slider more than the tighter rhythm gives back. That is the trade the floor was worth.

### Hit targets

24px is the floor for anything clickable in a panel, enforced in `controls.css` (see `Controls.md`). The four that were under it are listed there. A picture tile is about 78px tall, a segmented control 28, a switch 24, a slider's hit strip 24, and both the More row and a category header 28.

Rules that override something still living in `editor.css` are written with a `.e-panel` or `.editor` prefix rather than relying on source order: `Editor.tsx` imports `editor.css` LAST, so a same-specificity rule here would lose.
