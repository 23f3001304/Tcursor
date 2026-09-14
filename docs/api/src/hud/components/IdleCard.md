# src/hud/components/IdleCard.tsx

The idle HUD as a vertical card (owner, 2026-09-14, chosen over a one-row bar): the header with the brand and the window buttons, a wide webcam preview, the three sources as rows, the four source toggles as one labelled segmented row, and Record as the single accent button across the bottom. 360 wide. Replaces the 980 by 106 two-row bar (a title bar plus a row of a grip, a 52px webcam square, three ghost dropdowns, a 32px toggle group and a 34px Record).

The body has three states on one flip: the sources, the display picker (`TargetSheet`), and a panel (`panelBody` - Settings or Preferences). Since the card's height is fixed, the window never resizes for any of them.

## Toggles

```ts
export interface Toggles { camOn: boolean; micOn: boolean; sysOn: boolean; gameMode: boolean }
```

The four source switches, by `Hud`'s own state names. `onToggle` reports the key that was pressed; `Hud` flips the matching setter.

## IdleCard

```ts
export function IdleCard(p: {
  banner: string | null; exporting: boolean; pct: number;
  onOpenProject: () => void; onPreferences: () => void; onSettings: () => void; onMinimize: () => void; onClose: () => void;
  camRef: RefCallback<HTMLVideoElement>; camLive: boolean;
  cameras: DropOption[]; camId: string; onCam: (id: string) => void;
  targets: DisplayInfo[]; displayId: string; onTarget: (id: string) => void;
  mics: DropOption[]; micId: string; onMic: (id: string) => void;
  menu: string | null; onMenu: (id: string) => void;
  sheet: boolean; onSheet: (open: boolean) => void;
  panel: string | null; panelBody: ReactNode;
  toggles: Toggles; onToggle: (key: keyof Toggles) => void;
  onRecord: () => void;
}): JSX.Element
```

### Props

Every prop is `Hud`'s own truth; the card owns no state, so the take flow reads the same values this card shows.

- `banner` - the header's one flexible slot: a failed or OS-ended take, or an export failure, ellipsized with the full text on hover.
- `exporting` / `pct` - while an export runs from the HUD, the three app buttons (Open Project, Preferences, Settings) leave the header and Record shows `Exporting… {pct}%`, disabled.
- `onOpenProject`, `onPreferences`, `onSettings`, `onMinimize`, `onClose` - the header's five buttons, a hairline (`.winsep`) between the app three and the window two.
- `camRef` / `camLive` - `useWebcamPreview`'s callback ref and live flag, for the wide `CamTile`; the tile's on/off is `toggles.camOn`.
- `cameras` / `camId` / `onCam` and `mics` / `micId` / `onMic` - the two in-place `Dropdown` rows (`row` variant); `menu` / `onMenu` are `Hud`'s one-open-menu state, keyed `"cam"` and `"mic"`.
- `targets` / `displayId` / `onTarget` - the screen row shows the chosen target's title with its resolution and Primary badge as a sub-line (`parseTarget`), and opens the sheet.
- `sheet` / `onSheet` - whether the card body is flipped to `TargetSheet`. A pick in the sheet calls `onTarget` then `onSheet(false)`; Back calls `onSheet(false)` alone.
- `panel` / `panelBody` - which panel has the body (`"settings"`, `"preferences"` or `null`) and the element to render there. `panel` wins over `sheet`, and is the flip's key, so switching straight from one panel to the other frosts rather than cutting. *Why a node rather than the components:* `Hud` owns `panel`, both panels' `onClose`, and Preferences' `onUiChange` theme wiring - the card stays a body that shows what it is handed, with no settings imports of its own (the same split `StateSwap` uses for `idle`/`pill`).
- `onRecord` - `useRecordingFlow`'s `toggle`.

### Behavior

- **Rows are planes.** Each source row is a raised 46px plane (`.dd-row`) with a 34px icon lead, the device name and a chevron; the camera and mic chevrons point down and open a full-width menu under the row, the screen row's points right and flips the card.
- **The flip.** `AnimatePresence mode="wait"`, keyed `panel ?? (sheet ? "sheet" : "card")`, swaps `.card-body` between the three body states with a shallow version of the bar-to-pill frost (blur 10px, scale 0.97, a spring on the scale). Both the display picker (`.sheet`) and a panel (`.settings`, `settings.css`) are pinned to the body's own height (`--sheet-h`, 430px, `hud.css`), so no flip ever resizes the window - a panel taller than that scrolls inside itself, exactly as the picker's list does.
- **The header stays.** It sits above the body, so the brand, the app buttons and the window buttons are there in all three states; only the body swaps. Settings can therefore be swapped for Preferences (or the window minimized or closed) without going back first.
- **Toggles carry labels** (Camera, Mic, System, Compat) beside their icons, four equal segments in one plane; ON is the accent tint.
- **Record** is `RecordButton` at full width, 48px, the card's only accent.

### Used by

- `src/hud/Hud.tsx` - `StateSwap`'s `idle` child.
