# src/editor/panels/AiPanel.tsx

Left panel rendered when the "AI Director" rail tab is active. Displays a real Engine picker (Ollama models installed locally), the auto-edit run button, and a staggered Motion list summarizing what the AI director does.

## AiPanel

```tsx
export function AiPanel({ running, onRun, model, onChangeModel }: {
  running: boolean; onRun: () => void; model: string; onChangeModel: (v: string) => void;
}): JSX.Element
```

Renders the AI Director configuration panel and run control.

### Props

- `running: boolean` - whether the `aiAutoedit` IPC call is in flight. *Why:* disables the run button and switches it to a `Spin` + "Editing..." label to prevent re-submission and give feedback during a potentially multi-second local-LLM call.
- `onRun: () => void` - called when the Auto-edit button is clicked. *Why no model argument:* `Editor` already holds `doc.settings.ai_model` and reads it itself when calling `aiAutoedit`, so the run trigger and the model choice don't need to be threaded through the same callback.
- `model: string` - the persisted Ollama model name (`doc.settings.ai_model`; `""` means "no explicit choice yet"). *Why persisted rather than local state:* the choice should stick across closing/reopening the editor, same as every other panel's settings.
- `onChangeModel: (v: string) => void` - writes a new model choice back to `doc.settings.ai_model` (via `saveDocSettings` in `Editor`).

### Behavior

**Engine picker.**
On mount, fetches `listOllamaModels()` (a real IPC call hitting Ollama's `/api/tags`) into local `models` state. Falls back silently (`.catch(() => {})`) to an empty list if Ollama isn't running. The `Picker` shown to the user is populated from `models` when non-empty, or just `[current]` (the resolved current model) when the list hasn't loaded yet or Ollama is unreachable - so the control is never empty even offline. `current = model || FALLBACK_MODEL` (`FALLBACK_MODEL = "llama3.2"`, mirroring the Rust default in `ai::commands::ai_autoedit`), so an unset `ai_model` still shows a sensible selected value instead of a blank picker.

**No "Style" control.** The prior build had a second decorative "Style" dropdown (hardcoded "Demo") with nothing behind it in the backend - there is no server-side concept of edit "style" (the Ollama prompt is fixed in `ai::prompt::system_prompt`). It was removed rather than left as a non-functional stub.

**Run button.**
When `running` is false: renders `IconSparkles` + "Auto-edit" and calls `onRun` on click.
When `running` is true: renders `<Spin size={16}>` + "Editing..." and is `disabled`.

**Feature summary list.**
`SUMMARY` is a module-level array of three `[Icon, string]` pairs: zooms-on-clicks, camera punch-in, idle-gap trimming. Rendered as `motion.li` elements with a staggered entrance: `delay: 0.1 + i * 0.05`, `opacity: 0 -> 1`, `y: 5 -> 0`. *Why staggered:* communicates that the three items are distinct, sequential actions, and draws the eye down the list.

### Notes

- The module-level `SUMMARY` constant is fixed at build time; the items reflect the three actions currently implemented by the AI director (`aiAutoedit` via Ollama).
- `models` is the only local state; everything the user can actually change (`ai_model`) is a prop, round-tripped through `doc.settings`.
