# src/editor/AiPanel.tsx

Left panel rendered when the "AI Director" rail tab is active. Displays engine and style selectors (currently decorative dropdowns), the auto-edit run button, and a staggered Motion list summarizing what the AI director does. Stateless.

## AiPanel

```tsx
export function AiPanel({ running, onRun }: { running: boolean; onRun: () => void }): JSX.Element
```

Renders the AI Director configuration panel and run control.

### Props

- `running: boolean` - whether the `aiAutoedit` IPC call is in flight. *Why:* disables the run button and switches it to a `Spin` + "Editing..." label to prevent re-submission and give feedback during a potentially multi-second local-LLM call.
- `onRun: () => void` - called when the Auto-edit button is clicked. *Why:* the IPC call and its resulting doc update are handled by `Editor`; `AiPanel` only surfaces the trigger.

### Behavior

**Engine and style selectors.**
Two `<button class="e-sel">` elements labeled "Engine" (hardcoded value "Ollama") and "Style" (hardcoded value "Demo"). They are decorative stubs; clicking them does nothing. *Why rendered as buttons:* preserves layout and visual fidelity for the planned dropdown interaction.

**Run button.**
When `running` is false: renders `IconSparkles` + "Auto-edit" and calls `onRun` on click.
When `running` is true: renders `<Spin size={16}>` + "Editing..." and is `disabled`.

**Feature summary list.**
`SUMMARY` is a module-level array of three `[Icon, string]` pairs: zooms-on-clicks, camera punch-in, idle-gap trimming. Rendered as `motion.li` elements with a staggered entrance: `delay: 0.1 + i * 0.05`, `opacity: 0 -> 1`, `y: 5 -> 0`. *Why staggered:* communicates that the three items are distinct, sequential actions, and draws the eye down the list.

### Notes

- AiPanel has no local state and no effects.
- The module-level `SUMMARY` constant is fixed at build time; the items reflect the three actions currently implemented by the AI director (`aiAutoedit` via Ollama).
- The engine and style selectors will be wired to real IPC settings in a later milestone.
