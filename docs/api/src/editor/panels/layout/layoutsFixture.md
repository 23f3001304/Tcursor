# src/editor/panels/layout/layoutsFixture.tsx

The shared harness for the two Layouts-panel test files. `LayoutsPanel.test.tsx` covers what the panel edits (which layout is open, a knob change, a reset) and `LayoutPresetList.test.tsx` covers the saved-looks list; both mount the same panel against the same faked app config, so the mount, the fake doc and the query helpers live here rather than twice.

Not a `.test.` file, so vitest never collects it as a suite.

## BOLD

```ts
export const BOLD: AppearanceSettings
```

`DEFAULT_APPEARANCE` with one knob moved (`presenter.pad: 0.07`). Every "is this the look that got saved / applied" assertion compares against it, and the single changed field is what makes a wrongly-copied appearance visible.

## h

```ts
export const h: {
  container: HTMLDivElement;
  app: Settings;
  written: Settings[];
  saved: EditDoc["settings"][];
};
```

The one mutable holder both test files and the ipc mock read. It is an object rather than four `let` bindings because the mock factory reaches it through a dynamic import, and a live binding across that boundary is a re-export detail no test should have to know. `app` is what `getSettings` resolves with, `written` is every `setSettings` call, `saved` is every `onSaveSettings` the panel made.

## ipcMock

```ts
export const ipcMock: () => { getSettings: () => Promise<Settings>; setSettings: (s: Settings) => Promise<void> };
```

The module object each test file hands to `vi.mock("../../../shared/ipc", ...)`. Reads and writes `h`, so a test can stage `h.app` before mounting and assert on `h.written` after.

## docWith

```ts
export const docWith: (layout: string) => EditDoc
```

A minimal doc with one layout segment from 0 to 2000ms and default appearance. Cast rather than built in full: the panel reads `doc.layout` and `doc.settings` and nothing else, and a complete `EditDoc` literal would hide which two fields actually matter.

## byText

```ts
export const byText: <T extends HTMLElement>(sel: string, text: string) => T
```

The first element matching `sel` whose trimmed text is exactly `text`. Exact, not substring: "Save" and "Save current look" are two different buttons in this panel.

## rowNames

```ts
export const rowNames: () => (string | null)[]
```

The saved-looks rows in order, by name. The list's order is part of its contract (built-in Default first), so the assertion is on the whole array.

## mountPanel

```ts
export function mountPanel(): void
```

The `beforeEach`: resets `h`, clears `localStorage` (the `Disclosure` state persists there), and creates a fresh container and React root.

## unmountPanel

```ts
export function unmountPanel(): void
```

The `afterEach`: unmounts and removes the container, and clears `localStorage` again so a test that opened a disclosure cannot leak into the next file.

## show

```ts
export const show: (doc: EditDoc, timeMs?: number) => Promise<void>
```

Renders `LayoutsPanel` with that doc and playhead, inside `act`, awaiting the mount-time `getSettings`.

The panel is imported **lazily**, inside this function. The ipc mock's factory pulls this module in, so a static import of the panel - which imports ipc - would close the cycle and hang the runner.

### Used by

- `src/editor/panels/layout/LayoutsPanel.test.tsx`
- `src/editor/panels/layout/LayoutPresetList.test.tsx`
