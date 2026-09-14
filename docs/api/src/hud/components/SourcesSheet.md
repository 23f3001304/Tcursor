# src/hud/components/SourcesSheet.tsx

The take pill's Sources sheet: the three inputs, switchable without stopping the take (the HUD half of the 2026-09-14 mid-take source switching design). It hangs UNDER the pill on the same `.hud` surface, which grows by exactly its height (`useHudWindowSize`'s `SOURCES_HEIGHT`) and shrinks back when it closes, so the pill itself never changes width or moves.

The rows are the idle card's own pickers, verbatim: a `Dropdown` `row` for camera and mic, and a row that flips the sheet to `TargetSheet` for the display. Nothing here is a second way to read the same device list. Picking applies at once and closes the sheet - there is no Apply, and nothing to confirm.

The sheet arrives through the bar/pill swap's own frost (`StateSwap`'s `FROST`), and so does the flip to the display list inside it. Closing is a hard cut, for the reason `Dropdown`'s menu documents: the window is already gliding shut around it, and an exit animation under that is a clip, not a fade.

## SourcesSheet

```tsx
export function SourcesSheet(p: {
  targets: DisplayInfo[]; displayId: string; onTarget: (id: string) => void;
  cameras: DropOption[]; camId: string; onCam: (id: string) => void;
  mics: DropOption[]; micId: string; onMic: (id: string) => void;
  menu: string | null; onMenu: (id: string) => void;
  sheet: boolean; onSheet: (open: boolean) => void;
}): JSX.Element
```

### Inputs

- `targets` / `displayId` / `onTarget` - the capture targets, which one is selected, and what to do with a pick. `onTarget` is `Hud`'s `switchDisplay`, wrapped so it also closes the sheet.
- `cameras` / `camId` / `onCam` - the same `DropOption[]` the idle card's camera row gets (`Hud`'s `camOpts`). `onCam` is `switchCamera`.
- `mics` / `micId` / `onMic` - the same list the idle card's mic row gets. `onMic` is `switchMic`.
- `menu` / `onMenu` - which dropdown is open, `Hud`'s own state. The two rows claim the ids `"src-cam"` and `"src-mic"` - distinct from the idle card's `"cam"`/`"mic"` so an id can never open a row in both surfaces, and so `useHudWindowSize` can tell a take-time menu from a stale idle one.
- `sheet` / `onSheet` - whether the display list is showing instead of the three rows. Shared with the idle card's own flip, which is safe because the two are never on screen at the same time.

### Renders

- `.src-sheet` - the fixed-height box (`hud.css`; the same numbers `SOURCES_HEIGHT` is built from), frosting in on mount.
- `.src-body` - an `AnimatePresence mode="wait"` keyed on `sheet`, so the three rows and the target list cross-frost in place instead of the box jumping.
- Rows view: the display row (`.dd-row` with a right-pointing chevron, opening the list), then the camera and mic `Dropdown`s.
- List view: `TargetSheet`, whose back arrow returns to the rows and whose pick applies and closes.

### Behaviors (pinned by `src/hud/components/SourcesSheet.test.tsx`)

- Three rows, display first, each showing what is selected.
- The display row flips the sheet rather than opening a menu.
- Flipped, it is the same `TargetSheet` list the idle card uses; a pick reports the target id and closes the list.
- The camera and mic rows own separate menu ids, so one open menu never opens both, and a pick reports the DEVICE id, not the label.

### Used by

- `src/hud/Hud.tsx` - rendered under `TakeBar` inside `StateSwap`'s `pill` slot, only while `recording && !saving && sources`.
