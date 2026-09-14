# src/hud/components/TakeBar.tsx

The bar while a take runs: one 60px pill instead of the idle bar's title bar and row. The owner's rethink of the recording state (2026-09-14): the old recording bar kept the idle chrome (a 28px title bar with brand and window buttons, a grip, a 52px webcam square), mixed three control heights in one row (52 / 40 / 34), had two hero buttons (a raised Pause and an inverted Stop with a blinking dot) and put a 579 by 106 slab over the screen being recorded. The pill follows what Cap and Loom show while recording: one line, the timer as the hero, one red Stop, everything else quiet.

## TakeBar

```ts
export function TakeBar({ paused, saving, savePct, elapsed, err, micOn, sysOn, live, read, camRef, camOn, camLive, toggle, togglePause, sources, onSources }: {
  paused: boolean; saving: boolean; savePct: number; elapsed: number; err: string | null;
  micOn: boolean; sysOn: boolean; live: boolean; read: () => { mic: number; sys: number };
  camRef: RefCallback<HTMLVideoElement>; camOn: boolean; camLive: boolean;
  toggle: () => void; togglePause: () => void;
  sources: boolean; onSources: () => void;
}): JSX.Element
```

### Props

Every prop is `Hud`'s own truth; the pill owns no state.

- `paused: boolean` / `saving: boolean` / `savePct: number` - `useRecordingFlow`'s state. `saving` selects the saving pill (below); `Hud` only mounts `TakeBar` while `recording || saving`. `savePct` is the proxy transcode's own progress (`preprocess_project` emits it from ffmpeg's `-progress` stream), the one thing the editor waits for.
- `elapsed: number` - `useRecordingTimer(recording, paused)`, formatted by `formatTimer`.
- `err: string | null` - `useRecordingFlow`'s message for a degraded-but-running take (the webcam stopping, for instance). The idle bar shows it in the title bar's banner slot; the pill has no title bar, so it rides on the hover of a small orange `!` (`.take-warn`), the one non-button in the pill that keeps its pointer events.
- `micOn: boolean` / `sysOn: boolean` / `live: boolean` / `read` - the level slot: `RecMeter` is mounted only while `micOn || sysOn` (an audio source is on) and gets `live` and `read`; with both off there is no slot at all.
- `camRef` / `camOn` / `camLive` - `CamTile` as its `round` variant, the 44px self-view at the pill's left end, mounted only while `camOn`.
- `toggle: () => void` / `togglePause: () => void` - `useRecordingFlow`'s handlers, wired to Stop and Pause.
- `sources: boolean` / `onSources: () => void` - whether the Sources sheet under the pill is open, and the toggle for it (mid-take source switching, 2026-09-14). `sources` only lights the button and sets `aria-expanded`; the sheet itself is `Hud`'s, rendered as `TakeBar`'s sibling so the pill's own layout is untouched by it.

### Behavior

**A source that is off is not in the pill** (owner, 2026-09-14: "don't show camera and mic if they are turned off"). No off-glyph webcam circle, no struck mic: the pill shows the clock and the three buttons, plus the webcam circle while the camera is on and the level slot while an audio source is. The window is sized for exactly what is shown (`useHudWindowSize`'s `takeWidth`).

**Recording.** Left to right: the round webcam tile (camera on); the clock (`.take-clock`: a 9px accent dot that breathes on a 1.3s Motion loop, then the 17px tabular timer); the level slot (`RecMeter`, an audio source on: the 150 by 40 voice wave, or the word Paused); Sources as a quiet 42px round icon button (`Sliders`, titled "Sources") that opens the sheet under the pill; Pause as a second quiet 42px round icon button (`Pause` glyph, `Play` while paused, labelled Resume); Stop as the single accent-filled control (`StopSquare`, "Stop and save"). Both buttons press to 0.9 on a loose spring (stiffness 520, damping 14) that bounces back past rest - the pill's gooey feel. The Pause/Play glyph swaps through `AnimatePresence`: the old glyph shrinks away in 80ms, the new one springs in past full size on the same spring.

**Paused.** The `.paused` class: the dot holds still and goes dim, the timer dims, the slot shows Paused, and the Pause button lightens to say Resume is the live action. Nothing moves in the pill while the take is not moving.

**Saving.** The `.saving` class: the brand's idle wave (`IdleWave`, the same working indicator as the editor's `Spin`), the word Saving, the percentage, and a 3px progress line that stretches to fill the pill and springs to each new `savePct`. No buttons at all: there is nothing to do but wait, and the pill announces itself as a `role="status"`.

**Sources.** The one affordance the pill grew (owner ask, 2026-09-14: "can we give options to change display, microphone, camera mid recording"). It sits LEFT of Pause, not between Pause and Stop, so the two take controls stay adjacent and the destructive one stays at the end. Pressing it flips `Hud`'s `sources`, which mounts `SourcesSheet` under the pill and grows the window by the sheet's height; the button takes the `on` class (the hover plane, held) and reports `aria-expanded` while it is open. Absent entirely while `saving` - there is no take left to switch anything on.

**No minimize, no close.** Stop is the way out. Closing the window some other way (Alt+F4, the taskbar) still stops and saves through `Hud`'s `handleClose` and the Rust `CloseRequested` guard.

**Dragging.** The pill's root carries `data-tauri-drag-region`; `hud.css` gives every non-button child `pointer-events: none` (the warning glyph excepted) so a press anywhere but a button reaches it, the same trick the idle title bar uses.

**One width per take.** The slot is a fixed box and the saving line is `flex: 1`, so the pill is the same width paused or saving; `useHudWindowSize`'s `takeWidth` sizes the window for the sources this take shows (`TAKE_WIDTH`, 473, is the everything-on width, three 42px buttons included) and the window never resizes between Record and the editor. A mid-take warning (`err`) is the one thing that changes the width, by its 20px glyph and a gap. `Hud` pins the `.hud` surface to that width by inline style while the pill is the shown state, so the pill holds its shape while the window is still gliding down to it.

**Arriving and leaving.** `StateSwap` (its own doc) frosts the idle bar out and springs the pill in, and the window glides between the two sizes about its centre (`useHudWindowSize`, `morphWindow` with `"centre"`) at the same time.

### Used by

- `src/hud/Hud.tsx` - `StateSwap`'s `pill` child, shown while `recording || saving` in place of the idle card; the `.hud` root takes the `as-take` class once the pill is the shown state, which rounds its surface into the pill.
