# src/editor/shell/shellFixture.tsx

Test-only. The props builder the slot specs share.

## DOC

```ts
export const DOC: EditDoc
```

A doc with one zoom, enough for the inspectors to route to.

## shellProps

```ts
export function shellProps(over: Partial<ShellProps>): ShellProps
```

Every prop `Editor.tsx` hands the shell, at rest, with `over` applied.
