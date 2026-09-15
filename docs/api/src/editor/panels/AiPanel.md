# src/editor/panels/AiPanel.tsx

Left panel rendered when the "AI Director" rail tab is active: the Auto-edit run button, one sentence saying what it does, three bullets, and - demoted to the foot of the panel - a real Engine picker (Ollama models installed locally) with honest loading/empty/error states. Once a propose pass returns, the review sheet replaces the sentence and the bullets (M4 T4); the live "what I did" log of the one-shot flow is gone with that flow (M4 T5).

**Flow (look pass, 2026-09-14).** The owner dictated this panel's hierarchy: title, then the Auto-edit button, then "Automatically finds important moments and creates camera movement.", then the bullets Zooms on clicks / Removes idle time / Emphasizes important actions. So the order inverted - the ACTION is now the first thing under the header and the Engine picker, which used to open the panel, is one small labelled row at the bottom behind the panel's single allowed divider. Nothing about what the button does changed, and neither did the model-defaulting or the gating.

Three things got quieter with it: the run button is 36px (was 38) on a flat `--e-ai` with no gradient sheen and no drop shadow, hover is a brightness step; the bullets lost their 28px raised icon plates and sit at 12px `--e-mut` with the glyph at `--e-dim`; and the "What it does" / "Run" section heading is gone, since a sentence followed by three bullets does not need a label over it.

**Flow (panel pass, 2026-09-13, superseded above).** Two groups: the choice and the action (Engine, Auto-edit, progress, error), then the outcome under a heading. The log box and the error line became raised planes rather than bordered boxes - the error keeps its red text on a raised plane (its 3px accent edge went with every other accent bar on 2026-09-15).

## AiPanel

**The review sheet (M4 T4).** When `run` is non-null the panel's what-it-does copy (and the old live log) is replaced by `ReviewSheet` (`../director/review/ReviewSheet.md`) in the panel's own body. That placement is binding decision 3 of the M4 plan: the sheet is not a modal and not an overlay, so the stage, the transport and the timeline stay live while the user decides. The run button, its progress track and the Engine row are untouched and still sit above and below it.

```tsx
export function AiPanel({ running, exporting, error, onRun, model, onChangeModel, onAutoModel, progress, onClose,
  run, skipped, applying, previewId, onToggleItem, onPreviewItem, onApply, onDiscard }: {
  running: boolean;
  exporting: boolean;
  error: string | null; onRun: () => void; model: string; onChangeModel: (v: string) => void;
  onAutoModel: (v: string) => void;
  progress: { step: number; total: number } | null;
  onClose: () => void;
  run: AiRun | null; skipped: ReadonlySet<string>; applying: boolean; previewId: string | null;
  onToggleItem: (id: string) => void; onPreviewItem: (id: string) => void;
  onApply: () => void; onDiscard: () => void;
}): JSX.Element
```

Renders the AI Director configuration panel and run control.

### Props

- `running: boolean` - whether an AI Director pass is in flight, through all of its phases (`useAiRun`'s `running`: thinking, applying, and the pointer replay when it is on). *Why:* disables the run button and switches it to a `Spin` + "Thinking..." label to prevent re-submission and give feedback during a potentially multi-minute local-LLM call.
- `exporting: boolean` (bug-sweep-2 Task 8, L3) - also locks the run button, matching `Transport`'s wand and its own play/trim/aspect `locked` gate. A director pass mutating `edit.json` while an export renders from its own doc snapshot would silently diverge the preview/doc from the file being written; `Editor.tsx`'s `onRun` carries the same guard belt-and-braces.
- `error: string | null` - the last AI director run's failure message, or `null` when there's nothing to show. *Why inline rather than a toast:* the panel is already what the user is looking at right after clicking Auto-edit, so a `role="alert"` line under the button is enough.
- `onRun: () => void` - called when the Auto-edit button is clicked. *Why no model argument:* `Editor` already holds `doc.settings.ai_model` and reads it itself when fetching the plan, so the run trigger and the model choice don't need to be threaded through the same callback.
- `model: string` - the persisted Ollama model name (`doc.settings.ai_model`; `""` means "no explicit choice yet"). *Why persisted rather than local state:* the choice should stick across closing/reopening the editor, same as every other panel's settings.
- `onChangeModel: (v: string) => void` - writes a new model choice back to `doc.settings.ai_model` (via `saveDocSettings` in `Editor`) - the Engine picker's `onChange`, i.e. a deliberate user pick, which records an undo step.
- `onAutoModel: (v: string) => void` - the QUIET write counterpart (via `useDocSettings`'s `onAutoModel` in `Editor`, no undo step): used ONLY by the mount-time auto-default effect below, so opening the panel and landing on a sane default model never pushes a phantom undo step or an unasked-for disk write.
- `progress: { step: number; total: number } | null` - the pointer replay's live position (`useAiRun`'s `progress`, `src/editor/director/useAiRun.ts`). `null` while thinking and while applying; non-null only while the replay walks the applied edits, which happens only with `ui.ai_choreography` on.
- `run: AiRun | null` (M4 T4) - the last completed propose pass, or `null` for "nothing to review". Non-null IS "show the sheet"; it is also what hides the static sentence and bullets, since the sheet is the answer to the same question they were asking.
- `skipped: ReadonlySet<string>` / `previewId: string | null` / `applying: boolean` - which proposals are turned off, which one the stage is outlining, and whether an apply is in flight. All three are passed straight through to `ReviewSheet`; none of them is local state here, so the sheet survives the panel unmounting when the rail collapses.
- `onToggleItem` / `onPreviewItem` / `onApply` / `onDiscard` - the sheet's four actions, owned by `useAiRun` (`toggleItem` / `preview` / `apply` / `discard`). This panel routes them and decides nothing.
- `onClose: () => void` - (Task 26, since this panel adopted `PanelHeader`) `PanelHeader.onClose` isn't optional, but `AiPanel` renders for the `"ai"` tab - the router's own home/fallback - so there's nowhere meaningful to close TO. `EditorPanels` wires it to `() => setTab("ai")`, the same idiom every other panel uses; on this one it's a no-op (already on that tab), kept only so the header is wired consistently everywhere.

### Behavior

**Header (Task 26).** Renders via the shared `PanelHeader` - title "AI Director", one-line lede ("Auto-editing by a model running on this machine.", reworded in the look pass so it doesn't say the same thing as the sentence under the button). No `onReset` is passed (there's nothing here to reset to defaults).

**The sentence and the bullets.** `.e-ai-what` carries the owner's sentence verbatim; `SUMMARY` is the module-level array of three `[Icon, string]` pairs behind the bullets, in the owner's words and order (Zooms on clicks / Removes idle time / Emphasizes important actions) on the glyphs this panel already had - `IconZoomIn`, `IconCut`, `IconVideo`. They still enter staggered (`delay: 0.1 + i * 0.05`), which is what makes them read as three distinct actions rather than a paragraph in list clothing.

**Engine picker (loading/empty/error - Task 26; empty-state affordance - sweep-2 gate finding; demoted to the panel's foot in the look pass).**
It is now one `.e-ai-engine` row - an 11px "Engine" label, the control pushed to the right at 30px tall and at most 190px wide - sitting last in flow behind a `--e-divider` hairline. Last IN FLOW, not pinned with `margin-top: auto`: `Picker`'s menu only opens downward, and a row pinned to the very bottom of a `overflow-y: auto` panel would open it straight into the clip. The empty state's text shortens to "None found" to fit the narrower row; its Retry button, its `role="status"` and `noModelsTitle` are unchanged.

`models: OllamaModel[] | null` (M4: `{ name, vision }` per entry, not a bare name) - `null` means the `listOllamaModels()` fetch (a real IPC call hitting Ollama's `/api/tags`) is in flight; `loadModels` (a `useCallback`, re-run on mount and by the Retry button) resets it to `null` then calls the fetch, resolving to the real list or, on rejection, `[]`. Three renders of the Engine field, keyed on `models`:
- `null` -> a `Shimmer` skeleton (`.e-picker-shell`, sized to match the `Picker` button) instead of the control.
- `[]` (resolved empty, or the fetch rejected - e.g. Ollama isn't running) -> a disabled value-row, same `.e-picker-shell` footprint as the other two states plus `.e-picker-empty` (flex row, dim text) so the field never reads as literally blank: "No local models found" with a **Retry** button (re-runs `loadModels`) and a `title` (`noModelsTitle`) spelling out what to do. The `Picker` is not rendered at all in this state - it never presents a hardcoded placeholder model name as if it were actually installed and selectable.
- non-empty -> the real `Picker`, options built straight from `models` (no fallback entry). Each option's `label` is `engineDisplayName(m.name)` (`../director/engineName.ts`) - a short "Family Size (Quant)" name, e.g. `"Qwythos 9B (Q8)"` from `"hf.co/empero-ai/Qwythos-9B-Claude-Mythos-5-1M-GGUF:Q8_0"` - and `title: m.name` carries the full raw id on hover (Task 11, ux audit #16: the raw id used to wrap over two lines in this dropdown).

**The Vision badge (M4).** An option whose model reports vision gets `badge: "Vision"`, which `Picker` renders as a small chip after the label in both the closed button and the menu rows. *Why it earns the space:* whether the engine will actually LOOK at the recording is the one thing its name cannot say, and it changes what a run does - a text-only model plans from the transcript alone. A text-only model gets no badge at all rather than a "Text" one: the badge is a claim, not a decoration, and labelling the ordinary case would make the list noisier without telling anyone anything.

Once `models` resolves non-empty, a second effect defaults `model` to `models[0].name` via `onAutoModel` (NOT `onChangeModel` - see props above) whenever the saved choice is empty or no longer installed (`!models.some((m) => m.name === model)`) - so Auto-edit never sends a model name Ollama doesn't actually have pulled (which otherwise 404s), and doing so from mounting doesn't register as an undoable user edit. `current` prefers the saved model if it's actually installed, else the first installed model, else the raw `model` prop (only reachable transiently, since the Picker isn't shown while `models` is empty/null anyway). Manually changing the `Picker` itself still calls `onChangeModel`, which DOES record an undo step.

**Run button gating.** `disabled={running || exporting || !models?.length}` - besides the existing in-flight and export (bug-sweep-2 Task 8, L3) guards, the button is also disabled while there is definitively no local model to run against (loading or empty/error), since running would just fail immediately. `noModelsTitle` (non-`undefined` only once `models` has resolved to `[]`, not during the loading `null` state) is passed as the button's `title` too, so a disabled Auto-edit explains itself on hover instead of just sitting dead - the other gating reasons (`running`/`exporting`) are already self-explanatory from the button's own label or the rest of the UI.

**No "Style" control.** The prior build had a second decorative "Style" dropdown (hardcoded "Demo") with nothing behind it in the backend - there is no server-side concept of edit "style" (the Ollama prompt is fixed in `ai::prompt::system_prompt`). It was removed rather than left as a non-functional stub.

**Run button.**
When `running` is false: renders `IconSparkles` + "Auto-edit" and calls `onRun` on click. When `running` is true: renders `<Spin size={16}>` + "Thinking..." (or "Replaying... k of N" while `progress` is non-null) and is `disabled`. Carries `data-director-anchor="wand"` - alongside the Transport wand button, this is where `src/editor/director/targets.ts`'s `anchorPoint("wand")` looks for the fake pointer's start position; when this panel is open it wins (it sits earlier in DOM order than `Transport`), so the run visibly starts from the button the user is actually looking at.

**Progress track.**
While `running` and `progress` is non-null, a 2px `.e-ai-progress` track (`--e-ai` fill, `.e-ai-progress-fill`) renders directly under the run button, its width Motion-tweened to `(progress.step / progress.total) * 100%`. The track itself is wrapped in `AnimatePresence` (design/premium-pass D6, opacity + `y: -4 -> 0`, 0.14s) so it fades in/out around the run rather than popping.

**Error display (Retry added in M4 T4).**
The row now carries the raw failure string as its `title` and a **Retry** button (an underlined text button in the sentence, the same idiom the "None found" engine row uses) that calls `onRun` and is disabled while a run or an export is in flight. The friendly summary is what the user reads; the exact error is one hover away rather than lost behind it, which is binding decision 4's requirement for a failure.

When `error` is non-null, it's passed through `friendlyAiError` (`src/editor/director/friendlyAiError.ts`) into `{title, hint}` and rendered as a `role="alert"` paragraph (`.e-ai-err`): the title, plus a dimmer `.hint` line underneath when `hint` is non-null (the two failure modes an Ollama setup actually hits - no model installed, Ollama not running - get an actionable next step; anything else falls through as the raw message with no hint line, same as before this mapping existed). Shown independently of the log - a run can fail before producing any log lines at all. Also wrapped in its own `AnimatePresence` (same 0.14s opacity/y-4 tween, design/premium-pass D6).

**Feature summary list.**
Shown only while `run` is `null` - the review sheet replaces the sentence and the bullets, because a sheet of proposals is the answer to the question they were asking. Rendered as `motion.li` elements with a staggered entrance: `delay: 0.1 + i * 0.05`, `opacity: 0 -> 1`, `y: 5 -> 0`. *Why staggered:* communicates that the three items are distinct, sequential actions, and draws the eye down the list. The one-shot flow's live "what I did" log that used to take this block's place, and its `.e-ai-log` styles, went with that flow (M4 T5): the sheet says what will be done before it is done, which is the better narration.

### Notes

- The module-level `SUMMARY` constant is fixed at build time; the items reflect the three actions the AI director proposes (`aiPropose`, against a local Ollama model) and the sheet applies through `applyEditOp`.
- `models` is the only local state; everything the user can actually change (`ai_model`) is a prop, round-tripped through `doc.settings`.
