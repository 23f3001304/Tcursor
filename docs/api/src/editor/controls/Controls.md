# src/editor/controls/Controls.tsx

The barrel every panel and inspector imports its controls from, and - since the panel pass - the file that carries `controls/controls.css` into the bundle.

```ts
import "./controls.css";
export { Switch } from "./Switch";
export { Picker } from "./Picker";
export { Segmented } from "./Segmented";
export { NumberField } from "./NumberField";
export { Slider } from "./Slider";
export { Swatches } from "./Swatches";
export type { SwatchItem, SwatchVariant } from "./Swatches";
export { ColorInput, toHex, fromHex } from "./ColorInput";
export { TileRow, rowNextIndex } from "./TileRow";
export type { RowTile } from "./TileRow";
export { Disclosure, readDisclosure, writeDisclosure } from "./Disclosure";
```

`Spin`, `ResizeEdges` and `ConfirmDialog` are deliberately not re-exported here - they are not settings controls, and their two or three call sites import them directly.

**Why the stylesheet lives on the barrel.** Two files (`SpringControls`, `ExportDialog`) import `Slider`/`Picker` directly rather than through this barrel, so the import here is not what guarantees they are styled - the editor always mounts panels, which always import this barrel, which is enough. Putting it here keeps one obvious home for it next to the components it paints, instead of repeating the import in each control.

### controls.css

The shared controls' stylesheet. Four rules, the first three matching `panels.css`'s (see `PanelHeader.md`):

1. **Surfaces read by lightness, not strokes.** Every control rests on `--e-raised`, hovers to `--e-raised-hi` (a token this file defines - the same 3-to-4% step the elevation system already uses between `--e-surface` and `--e-surface-alt`), and drops to `--e-bg` for a groove or a well. The 1px borders that were on the picker button, the number field, the slider thumb, the switch track and the colour well are all gone.
2. **One accent** (`--e-primary`), only for an on/selected state. Colour swatches keep a neutral `--e-fg` ring, documented in `Swatches.md`.
3. **One motion language**, with nothing in this file animating a layout property in CSS.
4. **24px is the floor for anything you can click, drag or type into** (usability pass, 2026-09-13). The readout button and its edit field (19px), the slider's own hit strip (20px, and the 4px rail is only paint) and the switch track (23px) were all under it; all four are 24 now. Nothing in this sheet is smaller.

Rules moved here from `editor.css` (and deleted there): `.e-slider-*`, `.e-picker-*`, `.sw`, `.e-numfield`/`.e-numstep`, `.e-preset-*`/`.e-accent-*`/`.e-swatch-*`, `.e-colorpick`. Added by the panel pass: `.e-segmented`/`.e-segment*` (`Segmented.md`) and `.e-val`/`.e-val-edit`/`.e-fl-name` (`SliderValue.md`). Added by the usability pass: `.e-tilerow*`/`.e-rowtile*` (`TileRow.md`) and `.e-more*` (`Disclosure.md`). The tile-row scroller rules are shared with `panels.css`'s `.e-tile-strip`, so the reserved hover-only scrollbar lane is defined once.
