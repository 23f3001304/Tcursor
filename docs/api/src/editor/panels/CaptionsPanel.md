# src/editor/panels/CaptionsPanel.tsx

The Captions rail panel: one switch, and the sentence that qualifies it.

## CaptionsPanel

```tsx
export function CaptionsPanel({ settings, onChange, onClose }: {
  settings: ClickFxSettings; onChange: (v: ClickFxSettings) => void; onClose: () => void;
}): JSX.Element
```

`Show keystrokes on screen` writes `settings.captions` (`doc.settings.clickfx.captions`) through the same full-object `set()` idiom every other panel uses. Reset writes `false`, which is the Rust `ClickFxSettings::default()`.

The line under it says what this toggle is NOT: spoken captions (auto-transcription) are a later project, and this switch shows typed keystrokes. It is deliberately a sentence in the panel rather than a disabled "Transcribe" control - there is no backend behind such a control, and a dead control reads as broken.

### Look (panel pass, 2026-09-13)

One `.e-grp` holding the switch row and an `.e-hintline`, with no section heading: a heading over a single row is chrome with nothing to organise (`CameraRingField` is the other group that follows this rule). The qualifying sentence moved from `.e-lede` with inline margins to `.e-hintline`, which is the panel's one class for "a sentence, not a control".

### Used by

- `src/editor/EditorPanels.tsx` - the `tab === "captions"` panel.
