# src/editor/shell/ClassicShell.tsx

## ClassicShell

```tsx
export function ClassicShell(p: ShellProps)
```

The editor body in its classic composition plus a properties sidebar: the left icon rail (`Rail`) and
its panel slot (`EditorPanels`, the rail's tab, present only while one is open - see **The panel
column collapses** below), the stage column with its undo toast and the
director's overlay (`ShellStage`, `ShellStage.md`), a right column (`.e-props-side`, 360px at full size and
narrower on a laptop - see **Narrow windows** below) holding
`PropertiesSlot`, then `Transport` and `Timeline` underneath. Selection and panels are independent:
selecting a pill never replaces a panel, opening a panel never drops the selection (owner's call,
2026-09-13, after "clicking an effect replaces the left panel" read as bad flow).

The area/workspace shell (split tree, floating panels, workspaces, `Ctrl+1..9`, persisted layouts)
that briefly replaced this composition was vetoed by the owner on 2026-09-13 and is archived on
`archive/m1a-shell`. Everything it was built to host is here unchanged: the time remap's Time lane,
range selection, Cut / Speed / Remove silences and the output-time readout; the background assets;
the cut and speed inspectors.

Anything derivable from `doc` is spread rather than carried as its own `SlotProps` field - the stage's share of that lives in `ShellStage.md`, and `Transport` does the same here with `outOf(p.map, p.timeMs)` and `outDurMs(p.map)`. `Timeline` takes the same `p.map` too (Batch 4 T8, `Timeline.md`) - its Clips lane needs it for `clipOutMs`, and `ShellProps` already carried the one `TimeMap` every reader here needs, so no second field or a narrower "timeline slot" type was worth introducing for it.

### The AI review sheet passes through (M4 T4)

The shell routes the sheet's nine `ShellProps` fields and decides nothing about them. Eight (`aiRun`, `aiSkipped`, `aiApplying`, `aiPreviewId`, `onToggleItem`, `onPreviewItem`, `onApplyRun`, `onDiscardRun`) go to `EditorPanels`, since the sheet renders inside the AI panel's own body rather than over the stage. The ninth, `stageOutline`, reaches `Stage` through `ShellStage` as its `outline` prop. That split is the whole of the shell's involvement: the sheet is a panel, so it obeys the collapse rule below like every other panel, and a collapsed column simply hides it without losing it - the state lives in `Editor.tsx`.

### The panel column collapses (owner, 2026-09-14: "the left panel is not collapsible??")

The left column is now mounted only while `p.tab !== null`, inside an `AnimatePresence` of its own, and it uses the SAME two constants as the sidebar on the right: the `ARRIVE` spring in, the `LEAVE` tween out, zeroed under `useReducedMotion`. Both sides of the stage therefore arrive and leave in one voice, and the stage takes the freed width either way.

- **Three ways to close it** - pressing the rail tab that is already open, a panel's own close X (`setTab(null)`, see `EditorPanels.md`), and reopening the editor after it was left collapsed. **One way to open it** - pressing any rail tab.
- **`onTab`** is `(t) => p.setTab((cur) => nextTab(cur, t))`. The toggle rule is `nextTab` (`panelState.md`), not this file and not `Rail`, so the rail and the panels cannot disagree about what a press means. The functional setter matters: it reads the CURRENT tab at click time rather than the one closed over when the shell rendered.
- **`.e-panel-wrap` is the element that resizes**; `.e-panel-slot` inside it holds its full width from the first frame (`PANEL_W` here is what the animation aims at, and `--e-panel-w` is the same number in `editor.css` - both come from `useDensity()`, see **Narrow windows**). Exactly the `.e-props-side` / `.e-props-frost` pairing on the right, for exactly the same reason: a panel that re-typeset itself while the column slid would read as junk. The plane, the rounding and the clip live on the wrapper - the element actually changing size - with `.e-panel-wrap > .e-panel-slot { box-shadow: none }` so the two do not double up (`panels.css`).
- **`timeMs` is gated to two tabs** (render hygiene, extended in M5 T6): `p.tab === "camera" || p.tab === "captions" ? p.timeMs : 0`. Those are the only two panels that read the playhead - the Camera panel's keyframe field, and the Captions transcript's lit row - and every other tab gets a constant, so `EditorPanels`' `React.memo` can still skip a re-render on a tick. Anything that needs the true time regardless of tab reads `timeMsRef`, which is a ref and so never defeats memo on its own.
- **The rail never collapses.** It is the way back in; its 56px (48 on a laptop, `--e-rail-w`) is not what a cramped stage is short of.
- **No keyboard shortcut**, deliberately - the rail is always visible and one click away, and the editor's key table is for things that have no permanent affordance.

### The sidebar arrives and collapses (owner, 2026-09-14)

The properties column is **not mounted** while nothing is selected - the stage simply takes the
width. `selectedClip(p.doc, p.sel)` (see `PropertiesSlot.md`) is the whole condition, which is what
makes every existing deselection path collapse it for free: Escape (`keymap.ts`), an empty click on
the stage or the timeline, and deleting the selected clip all land on `sel = null`. The left panel is
untouched by any of it - a selection still never replaces it.

An `AnimatePresence` around the `<motion.aside>` owns the transition:

- **Arriving** - the aside animates `width` 0 -> `SIDE_W` on the owner's spring (`stiffness: 340,
  damping: 26`) while an inner `.e-props-frost` element frosts the content in: `blur(10px) -> 0` and
  opacity as plain tweens, the spring on `scale` alone. That pairing is `hud/components/StateSwap.tsx`'s
  interstate verbatim, so the sidebar arrives in the same voice as the HUD's bar-to-pill swap; a
  spring on a blur reads as flicker rather than bounce, which is why only the scale gets one.
- **Overshoot** - that spring is underdamped (ratio ~0.7), so it wants to pass its target by a few percent.
  `.e-props-side`'s `max-width` absorbs it: the column lands on the layout instead of pushing
  the stage narrower than its resting size and bouncing back. The spring is the one the owner asked
  for; the layout is what keeps it from overshooting past itself.
- **Narrow windows** - both column widths come from `useDensity()` (`density.md`), not from
  constants in this file: `SIDE_W` is `d.side` (360 / 320 / 288 / 240) and `PANEL_W` is `d.panel`
  (320 / 288 / 248). The stylesheet reads the identical numbers as `--e-side-w` / `--e-panel-w`,
  which `Editor.tsx` writes onto `.editor`, so the spring and the `max-width` that absorbs its
  overshoot can never aim at two different targets. *Why the sidebar steps at all:* the body spends
  the rail, the panel slot, three gaps and its own padding before the stage gets anything, so at the
  880px minimum window a flat 360 left the preview 94px wide, and every stage-anchored pill (the
  toast, the arrange hint, the director's Stop) then had nothing to fit into. *Why the panel slot
  steps too, which it used to refuse to:* the type steps with it, so a 288px column at scale 0.86
  is about 335 design-px of content - wider, in the only sense that matters, than the 320 the panels
  were drawn against. Collapsing the column, which is one rail press, is still the answer for the
  narrowest windows.
- **Leaving** - the same two animations in reverse on a 140ms tween. `AnimatePresence` keeps the
  outgoing aside's children exactly as they were when `sel` went null, so the inspector frosts out
  still showing the clip it was editing rather than blanking first.
- **Reduced motion** - `useReducedMotion()` drops the initial states and zeroes the exit, so the
  column simply appears and disappears.

`.e-props-frost` holds the full `--e-side-w` from its first frame, so the inspector's text is typeset
once and the column slides over finished content instead of reflowing it live (the rules are in
`editor.css`, in the `.e-props-side` block). It reads the same token the column's cap does, so a
narrow window shrinks the two together and the frost still holds one whole fixed width.
