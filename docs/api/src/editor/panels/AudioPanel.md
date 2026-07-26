# src/editor/panels/AudioPanel.tsx

The Audio rail panel: mic/system A/V sync offset, plus real per-track volume. Stateless (all values are props round-tripped through `doc.settings`).

## AudioPanel

```tsx
export function AudioPanel({
  offsetMs, onChangeOffset, micVol, onChangeMicVol, sysVol, onChangeSysVol, onClose,
}: {
  offsetMs: number; onChangeOffset: (v: number) => void;
  micVol: number; onChangeMicVol: (v: number) => void;
  sysVol: number; onChangeSysVol: (v: number) => void;
  onClose: () => void;
}): JSX.Element
```

Renders the three audio controls and writes straight back to `doc.settings` (via `Editor`'s `saveDocSettings`) on every change - there is no local/mocked state left in this panel.

### Props

- `offsetMs: number` / `onChangeOffset` - `doc.settings.audio_offset_ms` (ms, -300..300). Wired since before this change; unaffected by it.
- `micVol: number` / `onChangeMicVol` - `doc.settings.audio_mic_volume` (linear gain, 0..1.5; the slider shows/accepts 0..150%). *Why linear rather than a percentage type:* matches the Rust field exactly (`Settings.audio_mic_volume`) and ffmpeg's `volume` filter unit, so the UI-to-backend conversion is just `/100`/`*100`.
- `sysVol: number` / `onChangeSysVol` - `doc.settings.audio_sys_volume`. Same range and reasoning as `micVol`.
- `onClose: () => void` - closes the panel back to the AI tab.

### Behavior

Each slider is `min={0} max={150} step={5}`, displaying `Math.round(vol * 100)` and calling back with `v / 100`. *Why round-trip through a 0..150 integer instead of storing the 0..1.5 float directly in slider state:* keeps the displayed percentage exact (no floating-point display jitter) while the value written back to settings stays the same linear-gain float the backend expects.

### Removed (this change)

- Two "(Mocked)" volume sliders whose `onChange` was `() => {}` - replaced by the two real sliders above.
- A "Noise Suppression (Mocked)" three-button row with no `onClick` handlers at all and no backend (real noise suppression needs an actual DSP/audio-processing subsystem - out of scope, removed rather than left as a non-functional stub, since unlike auto-captions it isn't an already-planned future sub-project).

### Used by

- `src/editor/Editor.tsx` - the `tab === "audio"` panel; each `onChange*` calls `saveDocSettings({ ...doc.settings, <field>: v })`.
- `src-tauri/src/export/pipeline/audio_mux.rs` (`mux`) - the backend that actually applies `audio_mic_volume`/`audio_sys_volume` at export, via `RenderMeta.mic_volume`/`sys_volume`.
