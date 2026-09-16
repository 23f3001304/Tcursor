# src/editor/inspectors/TextContentField.tsx

The Content section of the Text inspector: the line, the optional second line, and the kind.

It is a file of its own for the reason `CaptionTextField.tsx` is - a region whose content IS text needs more controls than a region that only has numbers, and `TextInspector.tsx` was at its line budget with all of it inline. The split is by responsibility rather than by size: everything here answers "what does it say", everything left in the inspector answers "where, how big, and how does it arrive".

## TextContentField

```tsx
export function TextContentField({ item, onText, onSub, onKind }: {
  item: TextItem;
  onText: (text: string) => void;
  onSub: (sub: string | null) => void;
  onKind: (kind: TextKind) => void;
}): JSX.Element
```

Three controls, all fully controlled off `item` and all writing on `change`:

1. **The textarea** (`.e-itextarea`, two rows, resizable downward) for the main line. It commits every keystroke rather than on blur, which is what lets `useEditHistory`'s 400 ms coalescing turn a typing burst into one undo step; `CaptionTextField` blurs-to-commit instead because a caption's text carries word timings.
2. **The second line** (`.e-isubrow`) - a single-line input plus a clear button. The button sends `null`, not `""`, because the wire shape is `Option<String>` and `None` is what makes `lay_one` drop the sub row from the block's height entirely. It is disabled when there is nothing to clear.
3. **Kind** - a plain `Picker` over `TEXT_KIND_OPTIONS`.

Both text inputs stop `keydown` propagation, so typing a `t` or an `s` into a line cannot also fire the editor's own single-letter shortcuts.

### Why the callbacks and not the op

It takes three narrow callbacks rather than `onApply` so it stays a field and not a second inspector: the ONE `update_text` op shape lives in `TextInspector`'s `upd`, which is what makes the coalescing behaviour a property of the whole inspector rather than of each control.

### Used by

- `src/editor/inspectors/TextInspector.tsx` - the Content section.
