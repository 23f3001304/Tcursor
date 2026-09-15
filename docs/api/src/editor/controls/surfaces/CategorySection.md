# src/editor/controls/surfaces/CategorySection.tsx

One collapsible CATEGORY inside a panel: a quiet header row, and the items of that category under it. Added by the **arrangements pass** (2026-09-14) as the replacement for the horizontally scrolling tile strips in the Cursor and Background panels.

**What it replaced, and why.** The usability pass had turned every preset library into a sideways strip, which bought back a lot of panel height. The owner's read of the result: the strips hid most of every group behind a flick, the only place a tile's name appeared was a tooltip, and that tooltip was clipped by the strip that owned it. The ruling was *no horizontal movement anywhere in a panel* - a collapsible section per category instead, with the items wrapping under it.

**Why it is not `Disclosure`.** `Disclosure` is deliberately ONE row per panel, at its end, holding the controls nobody needs often (`Disclosure.md` says so in as many words). This is the opposite shape: several per panel, holding the panel's main choice. The look is shared on purpose - a chevron, an 11px word, no plane and no stroke - so a panel reads as one language rather than two kinds of collapsible.

**What keeps a stack of these from being a filing cabinet.** Two rules, both in this file:

1. The section holding the current selection **opens by itself** (`defaultOpenIndex`), and every other one starts closed. Opening a panel therefore shows exactly the group the user is already in.
2. A closed section that holds the selection **says what is chosen** at the right of its header. So a user can read the whole picker's state - "Playful: Cat" - without opening anything.

Neither rule writes to storage. Only a click does, which is what lets an untouched section keep following the selection from mount to mount.

## key

```ts
const key = (id: string) => `tcursor.panel.cat.${id}`
```

The localStorage key for one section's open state. The `tcursor.panel.` prefix is shared with `Disclosure`'s `tcursor.panel.more.<id>`, the `.cat.` segment keeps the two from ever colliding.

## TWEEN

```ts
const TWEEN = { type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }
```

The 0.16s content-swap tween from the m1a Look paragraph - the same one `Disclosure` uses, so a panel has one collapse language and not two.

## readCategory

```ts
export function readCategory(id: string): boolean | null
```

This section's remembered open state, or `null` when the user has never toggled it.

**`null` is the point.** A plain `false` would make "never touched" indistinguishable from "closed on purpose", and a section could then never follow the selection after its first mount. Storage can throw outright (a webview with site data blocked) and a panel must still render, so every failure reads as "nothing remembered" rather than as closed.

### Behaviors

- `reports null until the user has actually toggled the section`.
- `keys sections apart, so opening one category does not open another`.

## writeCategory

```ts
export function writeCategory(id: string, open: boolean): void
```

Remember this section's open state. Silent on failure, for the same reason as `readCategory`.

## defaultOpenIndex

```ts
export function defaultOpenIndex(holdsSelection: boolean[]): number
```

Which section a panel opens when nothing is remembered: the one holding the current selection, and the first when no section holds it. `holdsSelection[i]` is "section i contains what is chosen".

It always returns a valid index for a non-empty list, so a caller can render with a plain `defaultOpen={i === open}` and never has to special-case "nothing is selected". Callers: `CursorPackGrid` (over pack ids), `WallpaperTab` (over wallpaper ids plus the imported asset), `GradientTab` (over preset ids).

### Behaviors

- `opens the section holding the selection`.
- `falls back to the first section when nothing holds the selection`.
- `opens the FIRST holder if a caller ever passes two`.

## CategorySection

```tsx
export function CategorySection({ id, label, count, selectedName, defaultOpen, children }: {
  id: string;
  label: string;
  count: number;
  selectedName?: string | null;
  defaultOpen: boolean;
  children: ReactNode;
}): JSX.Element
```

### Props

- `id` - storage key suffix, one per category (`cursorpack.Playful`, `bg.Ribbons`, `bg.grad.Presets`, `bg.yourfile`). Categories are named by the data, so the key follows the data too.
- `count` - how many items are inside, shown as a dim badge. It is what gives a closed section a size, which is most of what a user needs to decide whether to open it.
- `selectedName` - the chosen item's name when THIS section holds the selection, else `null`. Rendered only while CLOSED: open, the ring on the tile itself says it better, and a header that repeats it competes with the thing it is describing.
- `defaultOpen` - open on a first mount, before the user has ever toggled this section. Read once, in the `useState` initializer, so a later re-render (a different wallpaper picked, say) never yanks a section shut under the user's hand.

### State and persistence

`readCategory(id) ?? defaultOpen` - a remembered state always wins, in both directions. Only `toggle` writes, so a section the user has never touched keeps deriving its state from the selection. The write is per `id`, so the Cursor panel's Playful section and the Background panel's Ribbons section remember apart.

### Accessibility

The header is a `<button aria-expanded aria-controls>` pointing at the body's `useId`. The body is genuinely UNMOUNTED while closed (`AnimatePresence`), not hidden, so a closed section's tiles are out of the tab order and out of the accessibility tree - which is what makes a six-section picker a handful of tab stops rather than sixty.

### Motion

Motion, never CSS keyframes: the chevron `animate`s its rotation and the body tweens `height`/`opacity` through `AnimatePresence`. Under `useReducedMotion` the timing becomes a zero-duration tween rather than a dropped animation, so `AnimatePresence` keeps owning the unmount either way.

### Styling

`.e-cat` / `.e-cat-btn` / `.e-cat-chev` / `.e-cat-label` / `.e-cat-count` / `.e-cat-sel` / `.e-cat-body` / `.e-cat-inner` live in `panels/panels.css` rather than `controls/controls.css`, because only panels stack categories and that file is where the rest of a panel's rhythm is stated. The control itself is shared so the two panels cannot drift apart. A stack of them sits in a `.e-grp.e-secstack` (2px between headers, 16px to whatever follows), because a library reads as one block rather than one section per group.

`.e-cat-sel` is the one place in this pass that may ellipsise, and only as a safety valve for an imported pack with a very long name - the name is never only there, it is captioned under its own tile one click away.

### Behaviors

- `starts closed when told to, with its content out of the tree (and so out of the tab order)`.
- `starts open when it is the section holding the selection`.
- `names its label and its item count in the header`.
- `shows the chosen item's name at the right only while closed`.
- `says nothing at the right when the selection is in another section`.
- `opens and closes on click, and remembers what it was left in`.
- `lets a remembered state beat the default, in both directions`.
- `writes nothing until the user toggles, so an untouched section keeps following the selection`.
- `wires the header to the body it controls`.

### Used by

- `src/editor/panels/cursor/CursorPackGrid.tsx` - one section per pack STYLE (`packCategories.md`).
- `src/editor/panels/background/WallpaperGrid.tsx` (`WallpaperTab`) - one per wallpaper group, plus "Your file".
- `src/editor/panels/background/GradientTab.tsx` - the gradient presets.
