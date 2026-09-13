# src/editor/stage/StageToolbar.tsx

The Stage's frame-tool strip: four icon buttons overlaid on the preview's top edge.

## StageToolbar

```tsx
export function StageToolbar({ tab, onTab, aspect, onAspect, aspectLocked }: {
  tab: Tab; onTab: (t: Tab) => void;
  aspect: Aspect; onAspect: (a: Aspect) => void;
  aspectLocked: boolean;
}): JSX.Element
```

Renders `.e-ftool` with four buttons: Aspect ratio, a divider, then Cursor, Captions, Camera.

### Investigation (Task 11, ux audit #15)

Before this task the component took **no props at all** and its `.on` state was a local
`useState` that just recolored the clicked icon - clicking never drove anything else, which is
exactly why the audit read the row as "unlabeled toggles with no visible effect." There is no live
"hide this layer" flag behind Cursor/Captions/Camera to wire to honestly either - captions in
particular only bake in at export time (`ClickFxSettings.captions`), never in the live preview, so
even a fully-wired toggle here couldn't make anything on the canvas change. Rather than fake three
more layer toggles, each button now does something real:

- **Aspect ratio** cycles the doc's aspect through the exact same `ASPECT_ORDER` sequence as
  `Transport`'s own aspect chip (`ASPECT_ORDER`/`ASPECT_LABEL` are exported from `Transport.tsx`
  and imported here, not duplicated) - a dramatic, unmistakably real stage-level effect: the canvas
  itself reshapes.
- **Cursor / Captions / Camera** each open that panel (`onTab("cursor"|"captions"|
  "camera")`) - clicking always produces an obvious, real change (the whole left panel swaps),
  which the brief's own escape hatch ("or the control must clearly read as off") covers for the
  case where there's no on-canvas layer to point at.

### Two kinds of button (fix round 1, controller ruling)

The reviewer flagged that treating Aspect and the Cursor/Captions/Camera trio as visually
identical toggles was itself misleading - Aspect really is a toggle (a live stage property with
two states you cycle between), but the other three are NAVIGATION: clicking one doesn't change
anything about the STAGE, it changes which panel is showing. Styling them the same way as
Aspect made them read as "toggles that happen not to do much," the same complaint this task set
out to fix. The row is now deliberately split into two groups:

- **Aspect** keeps the strong, latched look: `.on` = `aspect !== "source"` (a non-default aspect
  is engaged), styled with `--e-primary`'s red accent (`.e-tbtn.on`'s own language) - a real color
  + tinted background + border, not just a highlight. Tooltip is a state sentence: `"Aspect ratio:
  16:9 — click to switch to 9:16"`. Disabled (`aspectLocked`) while exporting or before a clip has
  loaded, mirroring `Transport`'s own `locked` gate on the identical action.
- **Cursor / Captions / Camera** get `.nav-on` instead of `.on` when `tab === id` - a soft neutral
  tint (`--e-soft` background, `--e-fg` text) that only MIRRORS the tab strip's own active look
  (`.e-ric.on`), never Aspect's colored "engaged" look, since nothing here is actually toggled -
  the button just happens to match whichever panel is currently open. Tooltip is a CONSTANT
  `"Open the {X} panel"` regardless of whether that panel is already open (no "panel is open"
  alternate phrasing - that dual-phrasing read as a toggle description, which is exactly what this
  group isn't). `aria-current` (not `aria-pressed`) marks the active one, matching the ARIA pattern
  for "this nav item points at where you already are," not a toggle button.
- A thin `.e-ftool-div` vertical divider (`--e-border2`, same token `Transport`'s own `.e-tdiv`
  uses) separates the two groups, so the row reads as "one live control, then three shortcuts"
  rather than four uniform buttons.

### Props

- `tab: Tab` / `onTab: (t: Tab) => void` - the active panel tab and the handler that opens one.
  `Tab` comes from `shell/panelTabs.tsx` since M1a (it used to live in the rail these
  buttons jumped). `onTab` is `EditorShell`'s quick-open: it sets the tab AND, if the workspace
  has no `panel` area to show it in, splits one off the stage - so a nav button here always has
  a visible effect no matter how the user has arranged their layout. It no longer clears
  `sel`/`aimOn`; selection is an independent axis now, which also retires the older `t !== tab`
  guard that existed only to stop a re-click from silently dropping the selection.
- `aspect: Aspect` / `onAspect: (a: Aspect) => void` - the doc's aspect ratio and its setter.
- `aspectLocked: boolean` - disables the aspect button; `Editor.tsx` passes `exporting || dur <= 0`.

### Behavior

Each button is a `motion.button` (`whileTap: scale 0.92`) with `title`/`aria-label`; see "Two
kinds of button" above for how `on`/`aria-*`/tooltip differ between Aspect and the nav trio.
