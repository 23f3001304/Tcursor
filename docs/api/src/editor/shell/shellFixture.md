# src/editor/shell/shellFixture.tsx

Test-only. The props builder the slot specs share.

## DOC

```ts
export const DOC: EditDoc
```

A doc with one zoom, enough for the inspectors to route to. It also carries an empty `captions` array and `DEFAULT_CAPTION_STYLE` in its settings (M5 T6): the Captions panel reads both unconditionally, and a fixture that stopped at the fields the older slots needed would crash the first spec that opened that tab rather than failing on something meaningful.

## shellProps

```ts
export function shellProps(over: Partial<ShellProps>): ShellProps
```

Every prop `Editor.tsx` hands the shell, at rest, with `over` applied. The AI review sheet's nine fields (M4 T4) rest at "no run": `aiRun: null`, an empty `aiSkipped`, `stageOutline: null` and no-op handlers, so a spec that does not care about the sheet sees the panel exactly as it looked before it existed.
