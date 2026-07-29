# src/editor/panels/AiPanel.tsx

Left panel rendered when the "AI Director" rail tab is active. Displays a real Engine picker (Ollama models installed locally), the auto-edit run button, an inline error line, and either a staggered Motion list summarizing what the AI director does (before the first run) or a live terminal-style log of what it actually did (once `log` has lines).

## AiPanel

```tsx
export function AiPanel({ running, error, log, onRun, model, onChangeModel }: {
  running: boolean; error: string | null; log: string[]; onRun: () => void; model: string; onChangeModel: (v: string) => void;
}): JSX.Element
```

Renders the AI Director configuration panel and run control.

### Props

- `running: boolean` - whether the `aiAutoedit` IPC call is in flight. *Why:* disables the run button and switches it to a `Spin` + "Directing…" label to prevent re-submission and give feedback during a potentially multi-second local-LLM call.
- `error: string | null` - the last `aiAutoedit` failure message, or `null` when there's nothing to show. *Why inline rather than a toast:* the panel is already what the user is looking at right after clicking Auto-edit, so a `role="alert"` line under the button is enough.
- `log: string[]` - the running director's live "what I did" narration, one line per completed step, appended to as `aiAutoedit` progresses. *Why a plain string array:* the panel only ever renders these as sequential text lines, so the caller doesn't need to hand over richer per-line metadata.
- `onRun: () => void` - called when the Auto-edit button is clicked. *Why no model argument:* `Editor` already holds `doc.settings.ai_model` and reads it itself when calling `aiAutoedit`, so the run trigger and the model choice don't need to be threaded through the same callback.
- `model: string` - the persisted Ollama model name (`doc.settings.ai_model`; `""` means "no explicit choice yet"). *Why persisted rather than local state:* the choice should stick across closing/reopening the editor, same as every other panel's settings.
- `onChangeModel: (v: string) => void` - writes a new model choice back to `doc.settings.ai_model` (via `saveDocSettings` in `Editor`).

### Behavior

**Engine picker.**
On mount, fetches `listOllamaModels()` (a real IPC call hitting Ollama's `/api/tags`) into local `models` state. Falls back silently (`.catch(() => {})`) to an empty list if Ollama isn't running. Once `models` loads, a second effect defaults `model` to `models[0]` via `onChangeModel` whenever the saved choice is empty or no longer installed (`models.length && !models.includes(model)`) - so Auto-edit never sends a model name Ollama doesn't actually have pulled (which otherwise 404s). The `Picker` shown to the user is populated from `models` when non-empty, or just `[current]` when the list hasn't loaded yet or Ollama is unreachable - so the control is never empty even offline. `current = models.includes(model) ? model : (models[0] ?? model ?? FALLBACK_MODEL)`: prefer the saved model if it's actually installed, else the first installed model, else the raw (possibly empty) `model` prop, else `FALLBACK_MODEL` (`"llama3.2"`) as the absolute last resort when Ollama is unreachable/has no models installed at all - when models ARE installed, the auto-default effect above means that last-resort branch is rarely what the user actually sees.

**No "Style" control.** The prior build had a second decorative "Style" dropdown (hardcoded "Demo") with nothing behind it in the backend - there is no server-side concept of edit "style" (the Ollama prompt is fixed in `ai::prompt::system_prompt`). It was removed rather than left as a non-functional stub.

**Run button.**
When `running` is false: renders `IconSparkles` + "Auto-edit" and calls `onRun` on click.
When `running` is true: renders `<Spin size={16}>` + "Directing…" and is `disabled`.

**Error display.**
When `error` is non-null, renders it as a `role="alert"` paragraph (`.e-ai-err`) directly under the run button. Shown independently of the log - a run can fail before producing any log lines at all.

**Live "what I did" log.**
While `log` is non-empty, it replaces the feature-summary list entirely with a terminal-style `.e-ai-log` block: each line is a `motion.div` that slides/fades in (`x: -8 -> 0`, `opacity: 0 -> 1`, a `0.24s` tween), and any line starting with `"✓"` gets the `done` class (a visual "completed step" treatment). *Why replace rather than append:* the log IS the "what I did" narration for the run that's in progress or just finished, so showing the static three-bullet summary alongside it would be redundant - the panel shows one or the other, keyed on `log.length > 0`.

**Feature summary list (fallback).**
Shown only while `log` is empty (no run yet). `SUMMARY` is a module-level array of three `[Icon, string]` pairs: zooms-on-clicks, camera punch-in, idle-gap trimming. Rendered as `motion.li` elements with a staggered entrance: `delay: 0.1 + i * 0.05`, `opacity: 0 -> 1`, `y: 5 -> 0`. *Why staggered:* communicates that the three items are distinct, sequential actions, and draws the eye down the list.

### Notes

- The module-level `SUMMARY` constant is fixed at build time; the items reflect the three actions currently implemented by the AI director (`aiAutoedit` via Ollama).
- `models` is the only local state; everything the user can actually change (`ai_model`) is a prop, round-tripped through `doc.settings`.
