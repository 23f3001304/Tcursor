# src/hud/components/icons.tsx

Collection of inline SVG icon components used throughout the HUD bar, dropdowns, settings panels, and window controls. Each is a zero-prop function component returning a single `<svg>` element that uses `currentColor` for stroke or fill, so every icon inherits its color from the nearest CSS `color` property without any prop threading.

## Monitor

18x18 viewport; a rounded rectangle (monitor body) with a stand path and horizontal base. *Rendered by:* `IdleCard` on the screen-capture `<Dropdown>` trigger and as the `<Monitor />` import alias.

## Mic

18x18 viewport; a rounded-rectangle capsule (microphone body) plus a curved stand path. *Rendered by:* `IdleCard` on the microphone `<Dropdown>` trigger, the mic toggle button, and inside the waveform meter row during recording.

## MinIcon

14x14 viewport; a single horizontal stroke used as the window minimize symbol. *Rendered by:* `IdleCard` on the minimize window-control button.

## CloseIcon

14x14 viewport; two diagonal strokes forming an X used as the window close symbol. *Rendered by:* `IdleCard` on the close window-control button.

## Chevron

11x11 viewport; a downward V-shaped path. *Rendered by:* `Dropdown` inside the trigger button to indicate open/close state.

## Check

13x13 viewport; a checkmark path (short lead, long diagonal). *Rendered by:* `Dropdown` beside the currently selected option in the open menu list.

## Camera

18x18 viewport; a stylized camera body path with a circular lens. *Rendered by:* `IdleCard` on the camera `<Dropdown>` trigger and on the camera toggle button when `camOn` is true, and as the camera-off icon overlay fallback label.

## CameraOff

18x18 viewport; the same camera body path with a diagonal strike-through line. *Rendered by:* `IdleCard` on the camera toggle button when `camOn` is false.

## MicOff

18x18 viewport; the microphone capsule and stand paths with a diagonal strike-through line. *Rendered by:* `IdleCard` on the microphone toggle button when `micOn` is false.

## Speaker

18x18 viewport; a speaker cone path with two arc paths indicating sound emission. *Rendered by:* `IdleCard` on the system-audio toggle button when `sysOn` is true.

## SpeakerOff

18x18 viewport; the speaker cone path with a single short arc and a diagonal strike-through. *Rendered by:* `IdleCard` on the system-audio toggle button when `sysOn` is false.

## Gear

18x18 viewport; a small circle (gear center) plus a notched ring path (gear teeth). *Rendered by:* `IdleCard` on the Settings window-control button in the titlebar.

## Back

16x16 viewport; a left-pointing chevron (single open-angle path). *Rendered by:* `Settings` and `Preferences` on the back-arrow button in their panel headers.

## Palette

18x18 viewport; a circle outline with three filled dot "paint spots" and a thumb-hole arc at the bottom. *Rendered by:* `IdleCard` on the Preferences button in the card header.

## FolderOpen

18x18 viewport; a single folder-body path (Feather-style outline). *Rendered by:* `IdleCard` on the Open Project button in the card header.

## Cpu

The compatibility-encoder toggle's icon: a chip with eight pins, since that toggle picks the legacy CPU recording path. Replaced the gamepad (a leftover of the toggle's old "game mode" name) on 2026-09-15.
## Pause

18x18 viewport, filled (not stroked): two rounded bars. *Rendered by:* `TakeBar` on the Pause button while the take runs.

## Play

18x18 viewport, filled: a rounded right-pointing triangle. *Rendered by:* `TakeBar` on the same button while paused, where it means Resume.

## StopSquare

16x16 viewport, filled: one rounded square. *Rendered by:* `TakeBar` on the accent-filled Stop button. Filled rather than stroked, like `Pause` and `Play`, so a small glyph reads at a glance on a 42px round button.

## Sliders

18x18 viewport; a mixer's three faders, each a stroked track with a knob at a different height. *Rendered by:* `TakeBar` on the Sources button, left of Pause. Stroked rather than filled (unlike the other three pill glyphs) because it reads as "the inputs feeding this take" only if the three tracks stay separable at 18px, which a solid silhouette loses.

## AppWindow

18x18 viewport; a window outline with a title-bar rule and two dots. *Rendered by:* `TargetSheet` on each window row of the "what to record" sheet.
