# src/editor/controls/Spin.tsx

The app's generic "working" indicator, used for preview, export and AI loading states. It is now a one-line wrapper around the wave motif's `IdleWave` - the brand's own wave with a breathing dot, rather than the rotating loader ring it used to render.

## Spin

```tsx
export function Spin({ size = 18 }: { size?: number }): JSX.Element
```

Renders `<IdleWave size={size} />`.

### Props

- `size?: number` - the wave's height in px; `IdleWave` makes the width `1.7 * size`. Defaults to 18. *Why a prop:* callers need different sizes (15 in TopBar's export button, 16 in AiPanel's run button).

### Why the API did not change

Keeping the same one-prop signature is the whole point: every existing `<Spin>` in the app inherits the wave motif without a call-site change, so the busy state is consistent across surfaces owned by different files. The panel-design benchmark calls this out twice - (c) item 2, that nobody in the category connects their brand mark to their loading states, and (e) item 6, that a generic spinner for AI processing is a cheap tell at exactly the moment the brand should feel most in control.

### Behavior

All of it lives in `IdleWave` (`src/lib/wave/ui/QuietWaves.tsx`): a slowly drifting low-amplitude sine with the dot breathing on the app-wide 2s cycle, drawn once and left still under `prefers-reduced-motion`. Colour is inherited from the surrounding text colour via `currentColor`, exactly as before.

### Used by

- `src/editor/shell/TopBar.tsx` - the export button while an export runs.
- `src/editor/panels/AiPanel.tsx` - the Direct button while a pass runs.

Two former callers moved to a more specific wave instead: `ExportProgress` now draws a determinate `SweepWave`, and the stage's empty state renders `StageEmpty`'s horizon wave.
