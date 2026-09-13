# src/editor/panels/AiPanel.tsx

Left panel rendered when the "AI Director" rail tab is active. Displays a real Engine picker (Ollama models installed locally) with honest loading/empty/error states, the auto-edit run button, an inline error line, and either a staggered Motion list summarizing what the AI director does (before the first run) or a live terminal-style log of what it actually did (once `log` has lines).

**Flow (panel pass, 2026-09-13).** Two groups: the choice and the action (Engine, Auto-edit, progress, error), then the outcome under a heading that reads "What it does" before a run and "Run" once there are log lines. Nothing moved and nothing was renamed; the grouping is what makes the run button read as belonging to the engine above it rather than floating between two unrelated blocks. The log box and the error line are raised planes now, not bordered boxes - the error keeps its red text and gains a 3px accent edge instead of a tinted, outlined card.

## AiPanel

```tsx
export function AiPanel({ running, exporting, error, log, onRun, model, onChangeModel, onAutoModel, progress, onClose }: {
  running: boolean;
  exporting: boolean;
  error: string | null; log: string[]; onRun: () => void; model: string; onChangeModel: (v: string) => void;
  onAutoModel: (v: string) => void;
  progress: { step: number; total: number } | null;
  onClose: () => void;
}): JSX.Element
```

Renders the AI Director configuration panel and run control.

### Props

- `running: boolean` - whether the AI director's plan-fetch-and-reveal pass (`onRun`, driven by `aiPlan` + a per-step `applyEditOp` in `Editor.tsx`/`useDirector.ts`) is in flight. *Why:* disables the run button and switches it to a `Spin` + "Directing…" label to prevent re-submission and give feedback during a potentially multi-second local-LLM call.
- `exporting: boolean` (bug-sweep-2 Task 8, L3) - also locks the run button, matching `Transport`'s wand and its own play/trim/aspect `locked` gate. A director pass mutating `edit.json` while an export renders from its own doc snapshot would silently diverge the preview/doc from the file being written; `Editor.tsx`'s `onRun` carries the same guard belt-and-braces.
- `error: string | null` - the last AI director run's failure message, or `null` when there's nothing to show. *Why inline rather than a toast:* the panel is already what the user is looking at right after clicking Auto-edit, so a `role="alert"` line under the button is enough.
- `log: string[]` - the running director's live "what I did" narration, one line per completed step, appended to as the reveal progresses. *Why a plain string array:* the panel only ever renders these as sequential text lines, so the caller doesn't need to hand over richer per-line metadata.
- `onRun: () => void` - called when the Auto-edit button is clicked. *Why no model argument:* `Editor` already holds `doc.settings.ai_model` and reads it itself when fetching the plan, so the run trigger and the model choice don't need to be threaded through the same callback.
- `model: string` - the persisted Ollama model name (`doc.settings.ai_model`; `""` means "no explicit choice yet"). *Why persisted rather than local state:* the choice should stick across closing/reopening the editor, same as every other panel's settings.
- `onChangeModel: (v: string) => void` - writes a new model choice back to `doc.settings.ai_model` (via `saveDocSettings` in `Editor`) - the Engine picker's `onChange`, i.e. a deliberate user pick, which records an undo step.
- `onAutoModel: (v: string) => void` - the QUIET write counterpart (via `useDocSettings`'s `onAutoModel` in `Editor`, no undo step): used ONLY by the mount-time auto-default effect below, so opening the panel and landing on a sane default model never pushes a phantom undo step or an unasked-for disk write.
- `progress: { step: number; total: number } | null` - the choreographed reveal's live position (`Editor`'s `director.progress`, from `src/editor/director/useDirector.ts`). `null` before a run starts and while the plan is still being fetched (the fake pointer is already visible and pressing the wand at that point, but step counting only starts once the plan resolves).
- `onClose: () => void` - (Task 26, since this panel adopted `PanelHeader`) `PanelHeader.onClose` isn't optional, but `AiPanel` renders for the `"ai"` tab - the router's own home/fallback - so there's nowhere meaningful to close TO. `EditorPanels` wires it to `() => setTab("ai")`, the same idiom every other panel uses; on this one it's a no-op (already on that tab), kept only so the header is wired consistently everywhere.

### Behavior

**Header (Task 26).** Renders via the shared `PanelHeader` (title "AI Director", the same lede as before) rather than a hand-rolled `<h2>`/`<p className="e-lede">` pair - consistent with every other panel. No `onReset` is passed (there's nothing here to reset to defaults).

**Engine picker (loading/empty/error - Task 26; empty-state affordance - sweep-2 gate finding).**
`models: string[] | null` - `null` means the `listOllamaModels()` fetch (a real IPC call hitting Ollama's `/api/tags`) is in flight; `loadModels` (a `useCallback`, re-run on mount and by the Retry button) resets it to `null` then calls the fetch, resolving to the real list or, on rejection, `[]`. Three renders of the Engine field, keyed on `models`:
- `null` -> a `Shimmer` skeleton (`.e-picker-shell`, sized to match the `Picker` button) instead of the control.
- `[]` (resolved empty, or the fetch rejected - e.g. Ollama isn't running) -> a disabled value-row, same `.e-picker-shell` footprint as the other two states plus `.e-picker-empty` (flex row, dim text) so the field never reads as literally blank: "No local models found" with a **Retry** button (re-runs `loadModels`) and a `title` (`noModelsTitle`) spelling out what to do. The `Picker` is not rendered at all in this state - it never presents a hardcoded placeholder model name as if it were actually installed and selectable.
- non-empty -> the real `Picker`, options built straight from `models` (no fallback entry). Each option's `label` is `engineDisplayName(m)` (`../director/engineName.ts`) - a short "Family Size (Quant)" name, e.g. `"Qwythos 9B (Q8)"` from `"hf.co/empero-ai/Qwythos-9B-Claude-Mythos-5-1M-GGUF:Q8_0"` - and `title: m` carries the full raw id on hover (Task 11, ux audit #16: the raw id used to wrap over two lines in this dropdown).

Once `models` resolves non-empty, a second effect defaults `model` to `models[0]` via `onAutoModel` (NOT `onChangeModel` - see props above) whenever the saved choice is empty or no longer installed (`models.length && !models.includes(model)`) - so Auto-edit never sends a model name Ollama doesn't actually have pulled (which otherwise 404s), and doing so from mounting doesn't register as an undoable user edit. `current = models?.includes(model) ? model : (models?.[0] ?? model)`: prefer the saved model if it's actually installed, else the first installed model, else the raw `model` prop (only reachable transiently, since the Picker isn't shown while `models` is empty/null anyway). Manually changing the `Picker` itself still calls `onChangeModel`, which DOES record an undo step.

**Run button gating.** `disabled={running || exporting || !models?.length}` - besides the existing in-flight and export (bug-sweep-2 Task 8, L3) guards, the button is also disabled while there is definitively no local model to run against (loading or empty/error), since running would just fail immediately. `noModelsTitle` (non-`undefined` only once `models` has resolved to `[]`, not during the loading `null` state) is passed as the button's `title` too, so a disabled Auto-edit explains itself on hover instead of just sitting dead - the other gating reasons (`running`/`exporting`) are already self-explanatory from the button's own label or the rest of the UI.

**No "Style" control.** The prior build had a second decorative "Style" dropdown (hardcoded "Demo") with nothing behind it in the backend - there is no server-side concept of edit "style" (the Ollama prompt is fixed in `ai::prompt::system_prompt`). It was removed rather than left as a non-functional stub.

**Run button.**
When `running` is false: renders `IconSparkles` + "Auto-edit" and calls `onRun` on click. When `running` is true: renders `<Spin size={16}>` + "Directing…" (or "Directing… k of N" once `progress` resolves) and is `disabled`. Carries `data-director-anchor="wand"` - alongside the Transport wand button, this is where `src/editor/director/targets.ts`'s `anchorPoint("wand")` looks for the fake pointer's start position; when this panel is open it wins (it sits earlier in DOM order than `Transport`), so the run visibly starts from the button the user is actually looking at.

**Progress track.**
While `running` and `progress` is non-null, a 2px `.e-ai-progress` track (`--e-ai` fill, `.e-ai-progress-fill`) renders directly under the run button, its width Motion-tweened to `(progress.step / progress.total) * 100%`. The track itself is wrapped in `AnimatePresence` (design/premium-pass D6, opacity + `y: -4 -> 0`, 0.14s) so it fades in/out around the run rather than popping.

**Error display.**
When `error` is non-null, it's passed through `friendlyAiError` (`src/editor/director/friendlyAiError.ts`) into `{title, hint}` and rendered as a `role="alert"` paragraph (`.e-ai-err`): the title, plus a dimmer `.hint` line underneath when `hint` is non-null (the two failure modes an Ollama setup actually hits - no model installed, Ollama not running - get an actionable next step; anything else falls through as the raw message with no hint line, same as before this mapping existed). Shown independently of the log - a run can fail before producing any log lines at all. Also wrapped in its own `AnimatePresence` (same 0.14s opacity/y-4 tween, design/premium-pass D6).

**Live "what I did" log.**
While `log` is non-empty, it replaces the feature-summary list entirely with a terminal-style `.e-ai-log` block: each line is a `motion.div` that slides/fades in (`x: -8 -> 0`, `opacity: 0 -> 1`, a `0.24s` tween), and any line starting with `"✓"` gets the `done` class (a visual "completed step" treatment). *Why replace rather than append:* the log IS the "what I did" narration for the run that's in progress or just finished, so showing the static three-bullet summary alongside it would be redundant - the panel shows one or the other, keyed on `log.length > 0`.

**Feature summary list (fallback).**
Shown only while `log` is empty (no run yet). `SUMMARY` is a module-level array of three `[Icon, string]` pairs: zooms-on-clicks, camera punch-in, idle-gap trimming. Rendered as `motion.li` elements with a staggered entrance: `delay: 0.1 + i * 0.05`, `opacity: 0 -> 1`, `y: 5 -> 0`. *Why staggered:* communicates that the three items are distinct, sequential actions, and draws the eye down the list.

### Notes

- The module-level `SUMMARY` constant is fixed at build time; the items reflect the three actions currently implemented by the AI director (planned via `aiPlan`, revealed through `applyEditOp`, both against a local Ollama model).
- `models` is the only local state; everything the user can actually change (`ai_model`) is a prop, round-tripped through `doc.settings`.
