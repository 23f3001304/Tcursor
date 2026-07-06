# src/hud/components/icons.tsx

Collection of inline SVG icon components used throughout the HUD bar, dropdowns, settings panels, and window controls. Each is a zero-prop function component returning a single `<svg>` element that uses `currentColor` for stroke or fill, so every icon inherits its color from the nearest CSS `color` property without any prop threading.

## Grip

10x20 viewport; two columns of three circles at fixed positions forming a drag-handle dot grid. *Rendered by:* `Hud` in the leftmost drag-handle div of the `.row`.

## Monitor

18x18 viewport; a rounded rectangle (monitor body) with a stand path and horizontal base. *Rendered by:* `Hud` on the screen-capture `<Dropdown>` trigger and as the `<Monitor />` import alias.

## Mic

18x18 viewport; a rounded-rectangle capsule (microphone body) plus a curved stand path. *Rendered by:* `Hud` on the microphone `<Dropdown>` trigger, the mic toggle button, and inside the waveform meter row during recording.

## MinIcon

14x14 viewport; a single horizontal stroke used as the window minimize symbol. *Rendered by:* `Hud` on the minimize window-control button.

## CloseIcon

14x14 viewport; two diagonal strokes forming an X used as the window close symbol. *Rendered by:* `Hud` on the close window-control button.

## Chevron

11x11 viewport; a downward V-shaped path. *Rendered by:* `Dropdown` inside the trigger button to indicate open/close state.

## Check

13x13 viewport; a checkmark path (short lead, long diagonal). *Rendered by:* `Dropdown` beside the currently selected option in the open menu list.

## Camera

18x18 viewport; a stylized camera body path with a circular lens. *Rendered by:* `Hud` on the camera `<Dropdown>` trigger and on the camera toggle button when `camOn` is true, and as the camera-off icon overlay fallback label.

## CameraOff

18x18 viewport; the same camera body path with a diagonal strike-through line. *Rendered by:* `Hud` on the camera toggle button when `camOn` is false.

## MicOff

18x18 viewport; the microphone capsule and stand paths with a diagonal strike-through line. *Rendered by:* `Hud` on the microphone toggle button when `micOn` is false.

## Speaker

18x18 viewport; a speaker cone path with two arc paths indicating sound emission. *Rendered by:* `Hud` on the system-audio toggle button when `sysOn` is true.

## SpeakerOff

18x18 viewport; the speaker cone path with a single short arc and a diagonal strike-through. *Rendered by:* `Hud` on the system-audio toggle button when `sysOn` is false.

## Gear

18x18 viewport; a small circle (gear center) plus a notched ring path (gear teeth). *Rendered by:* `Hud` on the Settings window-control button in the titlebar.

## Back

16x16 viewport; a left-pointing chevron (single open-angle path). *Rendered by:* `Settings` and `Preferences` on the back-arrow button in their panel headers.

## Palette

18x18 viewport; a circle outline with three filled dot "paint spots" and a thumb-hole arc at the bottom. *Rendered by:* `Hud` on the Preferences window-control button in the titlebar.

## Gamepad

18x18 viewport; a game controller body path with D-pad cross lines and two face-button dot strokes. *Rendered by:* `Hud` on the game-mode toggle button.
