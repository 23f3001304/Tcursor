# src/editor/panels/cursor/CursorPanel.tsx

Editor panel for the "Cursor" rail tab. Picks the cursor style (System / Enhanced / Hidden); for Enhanced it also selects the sprite pack, and holds the size, motion and click-bounce controls. Every Enhanced-only control is hidden for the System and Hidden styles.

**Flow (panel pass, 2026-09-13).** Five groups, in this order: **Style**, **Pack**, **Size**, **Motion** (Cursor Smoothness, Path Idealization, Motion Trail Blur, Motion Tilt), **Click** (the Click bounce switch and, directly under it, Click Bounce Intensity). Every setting and every setting NAME is the one that was there before; what changed is where they sit. Previously the bounce switch lived above the pack grid while the intensity it scales sat alone at the very bottom of the panel, four groups away - the owner's read of this panel was "the flow is incorrect", and this is that fix: what it is, how it looks, how it moves, then what a click does, with each switch directly above what it enables.

**Height (usability pass, 2026-09-13).** With all five groups open the panel was about 957px against a 620px slot, the pack grid alone accounting for 369 of it. Two changes, in this order:

1. **The packs became strips** (`CursorPackGrid.md`) - one 84px row per group instead of four wrapping rows. That is 285px, and it is the change that does the most work here.
2. **Motion and Click went under the panel's one `Disclosure`** - 216px more. They are set once and left; the pack and the size are what a user comes back to. Closed by default and remembered once opened.

Measured by rows at 320px, style Enhanced: padding 36, header 53, Style 57, gap 16, Pack 151, gap 16, Size 77, gap 16, the More row 28 - **450**, or 544 with an Imported strip as well. Style System or Hidden hides everything below Style, which is 190 at most (the honesty hint included).

## CursorPanel

```tsx
export function CursorPanel({ settings, onChange, onClose, osCursorInVideo = true, hasCursorLayer = false }: { settings: CursorSettings; onChange: (v: CursorSettings) => void; onClose: () => void; osCursorInVideo?: boolean }): JSX.Element
```

Renders the full Cursor settings panel.

### Props

- `settings: CursorSettings` - the current per-project cursor settings (`doc.settings.cursor`). *Why per-project, not global:* pack/size/style are edited post-record, alongside every other panel, and persist into `edit.json`.
- `onChange: (v: CursorSettings) => void` - called with the full next `CursorSettings` on every control change. *Why full object, not a patch:* mirrors every other panel's `set()` helper (`{ ...settings, [k]: v }`), keeping `Editor.tsx`'s `saveDocSettings({ ...doc.settings, cursor })` wiring uniform across panels.
- `onClose: () => void` - closes the panel (back to the AI tab).
- `osCursorInVideo?: boolean` - whether this recording's video already has the OS cursor baked in. When `false` and the style is `System`, the panel shows a one-line hint (wrapped in `AnimatePresence`, design/premium-pass D6 - fades in/out as the style picker changes, 0.14s opacity/y-4) that the cursor is re-created from the recorded path. *Why a hint and not a disabled option:* the fallback genuinely works (the renderer draws a plain arrow along the raw path), so `System` is a valid choice here - it just isn't the *original* cursor, and saying so avoids the "why does my cursor look different?" follow-up. *Why optional, defaulting to `true`:* the panel keeps rendering standalone - and in its own tests - exactly as before, with no hint.
- `hasCursorLayer?: boolean` (default `false`) - whether this recording captured the real OS cursor as its own layer (`cursor_layer`, see `events/track/cursorlayer.md`). When `true`, \"System\" IS the original cursor, so the \"re-created from the recorded path\" hint is suppressed; the hint now only appears for a pre-layer recording with neither a baked cursor nor a layer.

### Behavior

**Pack group.** The list fetch, the import call and the import error all moved to `CursorPackField.tsx` in the panel pass (see `CursorPackField.md`); this panel passes only `settings.pack` and a setter. That is what keeps this file a flat read of the panel's flow.

**Cursor style.** A 3-way `Segmented` (`STYLE_OPTS`) over all three `CursorStyle`s - `system` (keep the recording's baked-in OS cursor), `enhanced` (redraw a synthetic pointer), `hidden` (none) - writing `style` via `set`. It was a `Picker`: three exclusive states with one-word names is exactly what the benchmark says should be a segmented control, and it is the panel's primary choice, so it should not be hidden behind a dropdown. The segment labels are the bare state names because three descriptions do not fit one 320px row; the description each option carried (`"System - original cursor"`) is now the segment's tooltip, so nothing was lost.

Exposing `system` at all is what lets a clip recorded in System return to its original cursor in the editor; the old 2-way `Switch` could only reach `enhanced`/`hidden`. Everything below (the pack group and every slider) is gated behind `settings.style === "enhanced"`, since none of it affects the System or Hidden cursor.

**Click bounce.** The switch and its Click Bounce Intensity slider are one group, the slider `disabled` rather than removed while the switch is off - so the panel never changes height when the switch is flipped, and the control that is unavailable says so instead of vanishing.

**Reset.** Restores every field to `DEFAULT_CURSOR_SETTINGS` (below), including `pack: "default"` - does *not* touch `CursorPackField`'s fetched list (an imported pack stays visible in the grid after a reset; only the *selection* reverts to Default).

**Sliders.** Cursor Size (`0.4`-`3.0`), Cursor Smoothness (`0.0`-`1.0`), Path Idealization (`0.0`-`1.0`), Motion Trail Blur (`0`-`1`), Motion Tilt (`0`-`1`), Click Bounce Intensity (`0.1`-`1`) - each a direct `set(field, v)` on `Slider`'s `onChange`. *Cursor Smoothness* (`settings.smoothness`, default `0.6`) is the glide of each move between the recording's rests on the export side (`CursorSettings::smoothness_at` -> `Cursor::set_smoothness`, `export/cursor/path.rs`): `0` replays the recording's own timing and jitter, `1` is one clean eased stroke per move - and at any value the cursor is exactly where the hand rested and clicked. *Path Idealization* (`settings.path_idealize`, default `0.0`) straightens each of those moves toward the straight line between its two rests (`Cursor::set_idealize`); `0` leaves the recorded route untouched, and neither slider ever moves a rest or a click. *Motion Tilt* (`settings.tilt`, default `0.35`, step `0.05`, shown as `x.xx`) is how far a thrown cursor tips into its own travel and overshoots once coming back upright when it stops (`export/cursor/draw/tilt.rs`); it scales the 6-degree cap, and `0` switches the filter off. It sits last in the Motion group, after Motion Trail Blur, because the two are the same question asked twice - where the cursor has been, and how hard it is being thrown.

### Notes

- The pack picker itself lives in `CursorPackGrid.tsx`: built-in packs first under a dim group label, then imported ones, one scrolling strip each, every tile showing the pack's arrow and cycling its nine states on hover with the busy one animating. See `CursorPackGrid.md` and, for the tile and the glyph plate, `PackTile.md`.
- Selecting or importing a pack takes effect in both the export and the live editor preview: `Editor.tsx` passes `doc.settings.cursor` straight through to `Stage`, and `useEditorData`'s `cursorSprites` fetch is keyed on `doc?.settings.cursor.pack` so it re-decodes sprites exactly when the pack changes (not on unrelated edits).
- The panel's own lede is one line at hint size ("Shape, size and motion of the pointer."), which is the header rule the panel pass applied everywhere - see `PanelHeader.md`.

## DEFAULT_CURSOR_SETTINGS

```ts
export const DEFAULT_CURSOR_SETTINGS: CursorSettings
```

Mirrors the Rust `CursorSettings::default()` (`settings/cursor.rs`) field-for-field, including
`style: "system"` and `tilt: 0.35`. A Task 26 audit found this panel's Reset button wrote
`style: "enhanced"` here instead, silently diverging from the backend default on every reset;
extracted to a named, independently-testable constant (`CursorPanel.test.ts` asserts the whole
object, so a field added on one side and not the other fails immediately) so the two can't drift
apart unnoticed again.

### Used by

- `src/editor/Editor.tsx` - rendered for the `"cursor"` rail tab, wired to `doc.settings.cursor` / `saveDocSettings`
