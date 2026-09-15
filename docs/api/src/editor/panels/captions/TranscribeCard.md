# src/editor/panels/captions/TranscribeCard.tsx

The Captions panel's first block: pick a model, get it onto the machine, and turn the recording's own audio into a caption track. One primary button whose entire meaning comes from `modelCopy::actionFor`, so the card never reasons about the phase itself.

## TranscribeCard

```tsx
export function TranscribeCard({ models, style, phase, pct, error, onStyle, onDownload, onTranscribe }: {
  models: WhisperModelDto[]; style: CaptionStyle; phase: Phase; pct: number; error: string | null;
  onStyle: (s: CaptionStyle) => void; onDownload: (id: string) => void; onTranscribe: () => void;
}): JSX.Element
```

`model` and `language` live on `CaptionStyle` (ADDED-4: the panel edits them alongside the look, and both belong to the project), so both pickers write through the same `onStyle` the look controls use - one `saveDocSettings` shape for the whole tab.

### The language row

Whisper's English-only models cannot auto-detect, and `asr::models::resolve_model` refuses that pair with a message rather than silently transcribing as English. Neither `Picker` nor `Segmented` can disable a single option, so the card renders only the languages `languageOptions` marks enabled and puts the disabled one's reason on an `.e-hintline` underneath. The row hides entirely when that leaves one option. Picking an English-only model also writes `language: "en"`, so the document can never hold the pair the backend would refuse.

### The button and the bar

`actionFor` decides the label, the icon (download vs caption) and `disabled`; a busy phase shows `Spin` instead of an icon. While `phase !== "idle"` a determinate bar and a state line appear under it. The bar's width is the card's ONE stateful animation and so is the one thing Motion drives here (`SWAP_TWEEN`, the app's 160ms content beat); everything else is CSS and tokens.

`.e-caprun` overrides `.e-run`'s hue to `--e-wave`, and `.e-capbar-fill` matches: transcription reads the recording's AUDIO, and the wave hue is what every audio surface in the editor already uses. New tokens: none.

`error` renders on `.e-errline` exactly as it arrived - no re-wording, no summary, no retry button (the primary button is already the retry).

### Used by

- `src/editor/panels/captions/CaptionsPanel.tsx`
