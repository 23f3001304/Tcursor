# src/editor/shell/panelState.ts

The two rules behind the collapsible panel column (2026-09-14, owner: "the left panel is not collapsible??"). They live here rather than inline in the shell so they can be tested without mounting an editor, and so the rail and the panels cannot disagree about what a click means.

`null` is a real, reachable resting state now: the rail stays, the 320px column goes, and the stage takes the width.

## nextTab

```ts
export function nextTab(current: Tab | null, clicked: Tab): Tab | null
```

What clicking rail tab `clicked` should leave showing. Pressing the tab that is already open returns `null` (collapse); pressing any other returns that tab - **including from collapsed**, which is what makes the rail a toggle rather than a one-way door. `ClassicShell` builds its `onTab` out of exactly this, so the rail itself stays dumb: it reports which icon was pressed and nothing more.

## readPanelTab

```ts
export function readPanelTab(valid: readonly Tab[]): Tab | null
```

The panel the editor last had open, or `null` for "it was collapsed". `valid` is `TAB_IDS` (`panelTabs.tsx`) - passed in rather than imported so this file has no reason to know the tab list.

The empty string is the ONE stored value meaning collapsed, which is why it is written rather than the key being removed: a missing key has to mean "never used this editor", and that must open, not collapse. Every other failure reads as `"ai"` too - storage that throws outright (a webview with site data blocked), a value from a build whose tab no longer exists. **A user is never handed a collapsed editor they did not ask for.**

## writePanelTab

```ts
export function writePanelTab(tab: Tab | null): void
```

Remember the open panel, or that there is none (`""`). Silent on failure, for the same reason `readDisclosure`/`writeDisclosure` are: an editor must still open on a browser that refuses storage. Called from one `useEffect` in `Editor.tsx` keyed on `tab`, so every path that changes the tab persists it without any of them having to remember to.

### Storage key

`tcursor.editor.panel`, alongside the panels' own `tcursor.panel.more.*` (`Disclosure`) and `tcursor.panel.cat.*` (`CategorySection`). Per browser profile, not per project: which panel you had open is a habit, not a property of a recording.

### Used by

- `src/editor/Editor.tsx` - seeds `tab` state from `readPanelTab(TAB_IDS)`, persists it in an effect.
- `src/editor/shell/ClassicShell.tsx` - `nextTab`, for the rail's toggle.
