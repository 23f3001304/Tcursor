# src/editor/panels/background/BackgroundAssetCard.tsx

The Background panel's Custom row: the import tile that LEADS the row, and a card for whatever is already imported sitting beside it. Split from `BackgroundPanel.tsx` for that file's line budget.

## BackgroundAssetCard

```tsx
export function BackgroundAssetCard({ folder, asset, kind, onPick, onImported, onRemoved }: {
  folder: string;
  asset: string | null | undefined;
  kind: BackgroundKind;
  onPick: (kind: "image" | "video") => void;
  onImported: (info: BackgroundAssetInfo) => void;
  onRemoved: () => void;
}): JSX.Element
```

### Look

**The body of the LAST section of the Wallpapers tab (arrangements pass, 2026-09-14).** `WallpaperTab` wraps it in a `CategorySection` labelled "Your file", so it is one more group of the same library rather than a kind of its own - and the heading, the item count and the chosen file's name in a closed header all come from that section, not from here. This file renders only the row: `.e-bgassetwrap` around `.e-bgassetrow`, plus the error line. It was a hand-rolled `TileRow` row until the strips went; nothing here is a listbox, because it holds a button that opens a file dialog and a card that holds two buttons, neither of which is a choice in a list.

The import tile is `.e-bgadd`: a 64x36 raised plane with a plus-photo icon, an `aria-label` and a tooltip. It leads the row because an empty section is exactly what it is for. (`.e-upload`, the full-width version, stays where it still fits: the cursor-pack import.) The card itself is `.e-bgasset`, the same plane, the same one-step hover and the same 2px inset accent ring when chosen - because it IS one more way to choose a background, not a separate kind of object - laid out with a 56x32 thumbnail and two lines of text so it sits at the row's height. Remove is a 24px icon button that stays `--e-dim` until hovered: it is destructive, and it must not compete with the choice itself.

Motion is the panel's existing hint language (opacity + `y: -4` over 0.14s) for the card's enter/exit and the error line. `useReducedMotion` drops the Motion props entirely when the user asks for less.

### Behavior

**Describing what is there.** One `backgroundAssetInfo(folder, asset)` per `[folder, asset]`, into a three-state local: `undefined` while asking, `null` when the file is gone, the info when it resolves (see `assetSubtitle`). A rejected call is treated as `null` - from the card's point of view an asset it cannot describe is an asset that is not there.

**Import.** The Tauri dialog is filtered to the eight accepted extensions, so the common way to hit the backend's rejection is closed off before it happens; a rejection that does happen still shows as an `.e-errline` naming the accepted types. `busy` disables the button and swaps the label, matching `CursorPanel`'s import exactly.

**Select.** Clicking the card calls `onPick` with the asset's OWN kind (`assetKindOf`), which is what makes "pick a wallpaper, then come back" cost nothing: `settings.background.asset` was never cleared, so this re-asserts the kind and the file is already on disk. Disabled when the extension is unrecognised.

**Remove.** Calls `removeBackgroundAsset` and then `onRemoved`, which is where the panel clears `asset` and restores `kind: "mesh"`. A failing delete is swallowed: the file being gone already is the same outcome the user asked for. The command deliberately does not touch `edit.json` - the doc lives in the frontend, so the panel's own save is the only writer (see `bg_asset.md`).

### Used by

- `src/editor/panels/background/BackgroundPanel.tsx` - the Custom row, last in the Wallpapers tab's row stack.
