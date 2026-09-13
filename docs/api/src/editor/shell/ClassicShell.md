# src/editor/shell/ClassicShell.tsx

## ClassicShell

```tsx
export function ClassicShell(p: ShellProps)
```

The editor body in its classic composition plus a properties sidebar: the left icon rail (`Rail`) and its panel slot (`EditorPanels`, always the rail's tab), the stage with its undo toast and the director's overlay in `.e-stagetoast`, a fixed 360px right column (`.e-props-side`) holding `PropertiesSlot` (the selected clip's inspector, or one quiet sentence with nothing selected), then `Transport` and `Timeline` underneath. Selection and panels are independent: selecting a pill never replaces a panel, opening a panel never drops the selection (owner's call, 2026-09-13, after "clicking an effect replaces the left panel" read as bad flow).

The area/workspace shell (split tree, floating panels, workspaces, `Ctrl+1..9`, persisted layouts) that briefly replaced this composition was vetoed by the owner on 2026-09-13 and is archived on `archive/m1a-shell`. Everything it was built to host is here unchanged: the time remap's Time lane, range selection, Cut / Speed / Remove silences and the output-time readout; the background assets; the cut and speed inspectors.
