# src/editor/panels/HotkeysPanel.tsx

The Hotkeys rail panel: one switch, and the sentence that qualifies it. Renamed from `CaptionsPanel.tsx` (M5 T1, 2026-09-15): this panel edits `settings.clickfx.captions`, the hotkey-chord overlay toggle, not the spoken-caption feature - the old name and the "Captions" label were misleading now that a real Captions tab exists. `settings.clickfx.captions` itself keeps its name and meaning unchanged.

## HotkeysPanel

```tsx
export function HotkeysPanel({ settings, onChange, onClose }: {
  settings: ClickFxSettings; onChange: (v: ClickFxSettings) => void; onClose: () => void;
}): JSX.Element
```

`Show keystrokes on screen` writes `settings.captions` (`doc.settings.clickfx.captions`) through the same full-object `set()` idiom every other panel uses. Reset writes `false`, which is the Rust `ClickFxSettings::default()`.

The line under it now points to where spoken captions actually live: "Captions for what you say live on the Captions tab." It is deliberately a sentence in the panel rather than a disabled "Transcribe" control - there is no backend behind such a control yet, and a dead control reads as broken.

### Look (panel pass, 2026-09-13)

One `.e-grp` holding the switch row and an `.e-hintline`, with no section heading: a heading over a single row is chrome with nothing to organise (`CameraRingField` is the other group that follows this rule). The qualifying sentence moved from `.e-lede` with inline margins to `.e-hintline`, which is the panel's one class for "a sentence, not a control".

### Used by

- `src/editor/EditorPanels.tsx` - the `tab === "hotkeys"` panel.
