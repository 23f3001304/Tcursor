# src/editor/shell/ClassicShell.tsx

## ClassicShell

```tsx
export function ClassicShell(p: ShellProps)
```

The editor body in its classic composition plus a properties sidebar: the left icon rail (`Rail`) and
its panel slot (`EditorPanels`, the rail's tab, present only while one is open - see **The panel
column collapses** below), the stage with its undo toast and the
director's overlay in `.e-stagetoast`, a 360px right column (`.e-props-side`, narrower on a small
window - see **Narrow windows** below) holding
`PropertiesSlot`, then `Transport` and `Timeline` underneath. Selection and panels are independent:
selecting a pill never replaces a panel, opening a panel never drops the selection (owner's call,
2026-09-13, after "clicking an effect replaces the left panel" read as bad flow).

The area/workspace shell (split tree, floating panels, workspaces, `Ctrl+1..9`, persisted layouts)
that briefly replaced this composition was vetoed by the owner on 2026-09-13 and is archived on
`archive/m1a-shell`. Everything it was built to host is here unchanged: the time remap's Time lane,
range selection, Cut / Speed / Remove silences and the output-time readout; the background assets;
the cut and speed inspectors.

### The panel column collapses (owner, 2026-09-14: "the left panel is not collapsible??")

The left column is now mounted only while `p.tab !== null`, inside an `AnimatePresence` of its own, and it uses the SAME two constants as the sidebar on the right: the `ARRIVE` spring in, the `LEAVE` tween out, zeroed under `useReducedMotion`. Both sides of the stage therefore arrive and leave in one voice, and the stage takes the freed width either way.

- **Three ways to close it** - pressing the rail tab that is already open, a panel's own close X (`setTab(null)`, see `EditorPanels.md`), and reopening the editor after it was left collapsed. **One way to open it** - pressing any rail tab.
- **`onTab`** is `(t) => p.setTab((cur) => nextTab(cur, t))`. The toggle rule is `nextTab` (`panelState.md`), not this file and not `Rail`, so the rail and the panels cannot disagree about what a press means. The functional setter matters: it reads the CURRENT tab at click time rather than the one closed over when the shell rendered.
- **`.e-panel-wrap` is the element that resizes**; `.e-panel-slot` inside it keeps its fixed 320 (`PANEL_W` here is what the animation aims at, and matches that rule in `editor.css`). Exactly the `.e-props-side` / `.e-props-frost` pairing on the right, for exactly the same reason: a panel that re-typeset itself while the column slid would read as junk. The plane, the rounding and the clip live on the wrapper - the element actually changing size - with `.e-panel-wrap > .e-panel-slot { box-shadow: none }` so the two do not double up (`panels.css`).
- **The rail never collapses.** It is the way back in; 56px is not what a cramped stage is short of.
- **No keyboard shortcut**, deliberately - the rail is always visible and one click away, and the editor's key table is for things that have no permanent affordance.

### The sidebar arrives and collapses (owner, 2026-09-14)

The properties column is **not mounted** while nothing is selected - the stage simply takes the
width. `selectedClip(p.doc, p.sel)` (see `PropertiesSlot.md`) is the whole condition, which is what
makes every existing deselection path collapse it for free: Escape (`keymap.ts`), an empty click on
the stage or the timeline, and deleting the selected clip all land on `sel = null`. The left panel is
untouched by any of it - a selection still never replaces it.

An `AnimatePresence` around the `<motion.aside>` owns the transition:

- **Arriving** - the aside animates `width` 0 -> 360 on the owner's spring (`stiffness: 340,
  damping: 26`) while an inner `.e-props-frost` element frosts the content in: `blur(10px) -> 0` and
  opacity as plain tweens, the spring on `scale` alone. That pairing is `hud/components/StateSwap.tsx`'s
  interstate verbatim, so the sidebar arrives in the same voice as the HUD's bar-to-pill swap; a
  spring on a blur reads as flicker rather than bounce, which is why only the scale gets one.
- **Overshoot** - that spring is underdamped (ratio ~0.7), so it wants to pass 360 by a few percent.
  `.e-props-side`'s `max-width` absorbs it: the column lands on the layout instead of pushing
  the stage narrower than its resting size and bouncing back. The spring is the one the owner asked
  for; the layout is what keeps it from overshooting past itself.
- **Narrow windows** - that `max-width` is `--e-side-w` (`editor.css`), not a literal 360, and steps
  down to 320 under a 1180px window and 288 under 1020. `SIDE_W` here stays 360 - it is the width
  the spring aims at, and the cap is what the layout actually grants. *Why:* the body spends 56px on
  the rail, 320 on the panel slot, 30 on its gaps and 20 on its own padding before the stage gets
  anything, so at the 880px minimum window a flat 360 left the preview 94px wide, and every
  stage-anchored pill (the toast, the arrange hint, the director's Stop) then had nothing to fit
  into. The left panel slot keeps its 320 at every size: the panels are designed against that width,
  and narrowing them would only move the truncation. Collapsing it, which is now one rail press,
  is the answer for a narrow window instead.
- **Leaving** - the same two animations in reverse on a 140ms tween. `AnimatePresence` keeps the
  outgoing aside's children exactly as they were when `sel` went null, so the inspector frosts out
  still showing the clip it was editing rather than blanking first.
- **Reduced motion** - `useReducedMotion()` drops the initial states and zeroes the exit, so the
  column simply appears and disappears.

`.e-props-frost` holds the full `--e-side-w` from its first frame, so the inspector's text is typeset
once and the column slides over finished content instead of reflowing it live (the rules are in
`editor.css`, in the `.e-props-side` block). It reads the same token the column's cap does, so a
narrow window shrinks the two together and the frost still holds one whole fixed width.
